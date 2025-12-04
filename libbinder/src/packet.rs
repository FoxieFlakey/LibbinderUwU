use std::{mem, os::fd::BorrowedFd};

use enumflags2::BitFlags;
use libbinder_raw::{ObjectRef, ObjectRefLocal, Transaction, TransactionFlag, TransactionKernelManaged};

use crate::{command_buffer::{Command, CommandBuffer}, packet::builder::PacketBuilder, return_buffer::{ReturnBuffer, ReturnValue}};

pub mod builder;

// A friendly wrapper over transaction data for both incoming/outgoing
// and perform parsing too
//
// Its immutable, after constructed. Except few attributes such as
// flags, code, and target basically other than touching the buffers
#[derive(Clone)]
pub struct Packet<'binder> {
  binder_dev: BorrowedFd<'binder>,
  
  // Note Rust incapable of binding the first 'static to data_buffer
  // and second 'static to offset_buffer. The static lifetime is just
  // a placeholder. It is turned into local lifetime as needed
  //
  // Because of that, transaction has to come before the buffers to
  // be dropped
  transaction: Transaction<'binder, 'static, 'static>,
  
  data_buffer: Vec<u8>,
  offset_buffer: Vec<usize>
}

impl<'binder> Into<PacketBuilder<'binder>> for Packet<'binder> {
  fn into(mut self) -> PacketBuilder<'binder> {
    let common = self.transaction.get_common();
    match &self.transaction {
      Transaction::KernelManaged(x) => {
        // The buffer associated with it is kernel managed
        // we have to copy...
        
        self.data_buffer.copy_from_slice(x.get_data().data_slice);
        self.offset_buffer.copy_from_slice(x.get_data().offsets);
      }
      
      Transaction::NotKernelManaged(_x) => {
        // The buffer associated with it, is managed by Packet, so nothing need
        // to be done
      }
    };
    
    PacketBuilder {
      binder_dev: Some(self.binder_dev),
      code: Some(common.code),
      data_buffer: self.data_buffer,
      offsets_buffer: self.offset_buffer,
      flags: Some(common.flags)
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub enum PacketSendError {
  // Transaction cannot be sent to target
  Failed,
  
  // Transaction did sent to target, but
  // it died before sending reply
  DeadTarget
}

impl<'binder> Packet<'binder> {
  // SAFETY: The 'bytes' has to be from kernel from the correct binder_dev
  // and the bytes assumed to be from BR_TRANSACTION/BR_REPLY
  //
  // The 'bytes' alignment can be unaligned, and its fine
  //
  // For more accurate one see libbinder-raw/src/transaction/kernel_managed.rs
  //
  // The .0 is Some, incase its not a reply and indicates which object the transaction
  // acted on
  pub(crate) unsafe fn from_bytes(binder_dev: BorrowedFd<'binder>, bytes: &[u8], is_reply: bool) -> (Option<ObjectRefLocal>, Self) {
    // SAFETY: Caller met the requirement
    let transaction = Transaction::KernelManaged(unsafe { TransactionKernelManaged::from_bytes(binder_dev, bytes, is_reply) });
    
    (
      if is_reply {
        None
      } else if let ObjectRef::Local(reference) = transaction.get_common().target.clone() {
        Some(reference)
      } else {
        panic!("BR_TRANSACTION returns remote reference!");
      },
      
      Self {
        binder_dev,
        data_buffer: Vec::new(),
        offset_buffer: Vec::new(),
        transaction
      }
    )
  }
  
  pub fn set_code(&mut self, code: u32) {
    self.transaction.with_common_mut(|common| {
      common.code = code;
    });
  }
  
  #[expect(unused)]
  pub fn set_flags(&mut self, flags: BitFlags<TransactionFlag>) {
    self.transaction.with_common_mut(|common| {
      common.flags = flags;
    });
  }
  
  pub fn get_code(&self) -> u32 {
    self.transaction.get_common().code
  }
  
  pub fn send_as_reply(&self) {
    CommandBuffer::new(self.binder_dev)
      .enqueue_command(Command::SendReply(self.transaction.clone()))
      .exec(Some(&mut ReturnBuffer::new(self.binder_dev, 4096)));
  }
  
  // If the transaction doesn't result anything. None is retured
  // else error if there error
  pub fn send(&self, target: ObjectRef) -> Result<Option<Packet<'binder>>, PacketSendError> {
    if matches!(target, ObjectRef::Local(_)) {
      todo!("Handle local transaction");
    }
    
    let mut transaction = self.transaction.clone();
    let mut ret_buf = ReturnBuffer::new(self.binder_dev, 4096);
    transaction.with_common_mut(|x| x.target = target);
    
    // Send transaction
    CommandBuffer::new(self.binder_dev)
      .enqueue_command(Command::SendTransaction(transaction))
      .exec(None);
    
    // Read reply
    CommandBuffer::new(self.binder_dev)
      .exec(Some(&mut ret_buf));
    
    let mut latest_reply = None;
    let mut is_dead = false;
    let mut cant_be_sent = false;
    ret_buf.get_parsed()
      .iter()
      .rev()
      .for_each(|val| {
        match val {
          ReturnValue::Noop => (),
          ReturnValue::Reply(reply) => {
            if mem::replace(&mut latest_reply, Some(Some(reply.clone()))).is_none() {
              panic!("There were multiple responses to one transaction");
            }
          },
          ReturnValue::DeadReply => {
            is_dead = true;
          }
          ReturnValue::TransactionFailed => {
            cant_be_sent = true;
          }
          ReturnValue::TransactionComplete => {
            if mem::replace(&mut latest_reply, Some(None)).is_none() {
              panic!("There were multiple responses to one transaction");
            }
          },
          _ => panic!("unhandled")
        }
      });
    
    match (latest_reply, is_dead, cant_be_sent) {
      (Some(reply), false, false) => Ok(reply),
      (None, true, false) => Err(PacketSendError::DeadTarget),
      (None, false, true) => Err(PacketSendError::Failed),
      (None, false, false) => panic!("did not get any response for transaction from kernel"),
      _ => panic!("ambigious condition")
    }
  }
}


