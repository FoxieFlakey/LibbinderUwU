use std::{io, mem, os::fd::BorrowedFd};

use enumflags2::BitFlags;
use libbinder_raw::{object::reference::{ObjectRef, ObjectRefLocal, ObjectRefRemote}, transaction::{Transaction, TransactionFlag, TransactionKernelManaged}, types::Type};

use crate::{command_buffer::{Command, CommandBuffer}, formats::ReadFormat, packet::{builder::PacketBuilder, reader::Reader}, return_buffer::{ReturnBuffer, ReturnValue}};

pub mod builder;
pub mod reader;
pub mod writer;

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
  
  pub(self) data_buffer: Vec<u8>,
  pub(self) offset_buffer: Vec<usize>
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
      binder_dev: self.binder_dev,
      code: Some(common.code),
      data_buffer: self.data_buffer,
      offsets_buffer: self.offset_buffer,
      flags: Some(common.flags)
    }
  }
}

#[derive(Debug)]
pub enum PacketSendError {
  // Transaction cannot be sent to target
  Failed,
  
  // Kernel sent an error
  Error(io::Error),
  
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
  
  pub fn reader<'packet, Format: ReadFormat<'packet>>(&'packet self, format: Format) -> Reader<'packet, 'binder, Format> {
    Reader::new(self, format)
  }
  
  pub fn set_code(&mut self, code: u32) {
    self.transaction.with_common_mut(|common| {
      common.code = code;
    });
  }
  
  pub fn set_flags(&mut self, flags: BitFlags<TransactionFlag>) {
    self.transaction.with_common_mut(|common| {
      common.flags = flags;
    });
  }
  
  pub fn get_code(&self) -> u32 {
    self.transaction.get_common().code
  }
  
  pub fn get_binder_dev(&self) -> BorrowedFd<'binder> {
    self.binder_dev
  }
  
  pub fn send_as_reply(&self) -> Result<(), PacketSendError> {
    // Send reply
    CommandBuffer::new(self.binder_dev)
      .enqueue_command(Command::SendReply(self.transaction.clone()))
      .exec_always_block(None)
      .unwrap();
    
    // Read reply result
    let mut ret_buffer = ReturnBuffer::new(self.binder_dev, 64);
    CommandBuffer::new(self.binder_dev)
      .exec_always_block(Some(&mut ret_buffer))
      .unwrap();
    
    self.handle_result()
      .map(|x| {
        assert!(x.is_none(), "kernel sent a reply for a reply, when not expected")
      })
  }
  
  fn handle_result(&self) -> Result<Option<Packet<'binder>>, PacketSendError> {
    // Read reply
    let mut ret_buf = ReturnBuffer::new(self.binder_dev, 256);
    CommandBuffer::new(self.binder_dev)
      .exec_always_block(Some(&mut ret_buf))
      .unwrap();
    
    let mut latest_reply = None;
    let mut is_dead = false;
    let mut cant_be_sent = false;
    let mut completed = false;
    ret_buf.get_parsed()
      .iter()
      .rev()
      .for_each(|val| {
        match val {
          ReturnValue::Noop => (),
          ReturnValue::Reply(reply) => {
            if mem::replace(&mut latest_reply, Some(reply.clone())).is_some() {
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
            completed = true;
          },
          _ => panic!("unhandled")
        }
      });
    
    let latest_reply_is_some = latest_reply.is_some(); 
    match (latest_reply, is_dead, cant_be_sent, completed) {
      (reply, false, false, true) => Ok(reply),
      (None, true, false, true) => Err(PacketSendError::DeadTarget),
      (None, false, true, false) => Err(PacketSendError::Failed),
      (None, false, false, false) => panic!("did not get any response for transaction from kernel"),
      _ => panic!("ambigious condition, latest_reply.is_some = {latest_reply_is_some}, is_dead = {is_dead}, cant_be_sent = {cant_be_sent}, completed = {completed}")
    }
  }
  
  pub fn iter_references(&self) -> impl Iterator<Item = (usize, ObjectRef)> {
    self.transaction.get_common().offsets
      .iter()
      .map(|&x| {
        (x, Type::from_bytes(&self.transaction.get_common().data_slice[x..x+Type::bytes_needed()]))
      })
      .map(|(offset, obj_ty)| {
        let bytes = &self.transaction.get_common().data_slice[offset..offset+obj_ty.type_size_with_header()];
        match obj_ty {
          Type::LocalReference | Type::RemoteReference => {
            (offset, ObjectRef::try_from_bytes(bytes).unwrap())
          }
          _ => panic!("unexpected")
        }
      })
  }
  
  // If the transaction doesn't result anything. None is retured
  // else error if there error
  pub fn send(&self, target: ObjectRefRemote) -> Result<Packet<'binder>, PacketSendError> {
    let mut transaction = self.transaction.clone();
    transaction.with_common_mut(|x| x.target = ObjectRef::Remote(target));
    
    // Send transaction
    CommandBuffer::new(self.binder_dev)
      .enqueue_command(Command::SendTransaction(transaction))
      .exec_always_block(None)
      .unwrap();
    
    self.handle_result()
      .map(|x| x.expect("reply was expected but kernel did not send any"))
  }
}


