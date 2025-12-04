use std::os::fd::BorrowedFd;

use enumflags2::BitFlags;
use libbinder_raw::{ObjectRef, ObjectRefLocal, Transaction, TransactionFlag, TransactionKernelManaged};

use crate::{command_buffer::{Command, CommandBuffer}, packet::builder::PacketBuilder, return_buffer::ReturnBuffer};

pub mod builder;

// A friendly wrapper over transaction data for both incoming/outgoing
// and perform parsing too
//
// Its immutable, after constructed. Except few attributes such as
// flags, code, and target basically other than touching the buffers
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
  
  #[expect(unused)]
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
  
  pub fn send(&self, target: ObjectRef) -> Packet<'binder> {
    if matches!(target, ObjectRef::Local(_)) {
      todo!("Handle local transaction");
    }
    
    let mut transaction = self.transaction.clone();
    transaction.with_common_mut(|x| x.target = target);
    CommandBuffer::new(self.binder_dev)
      .enqueue_command(Command::SendTransaction(transaction))
      .exec(Some(&mut ReturnBuffer::new(self.binder_dev, 4096)));
    todo!();
  }
}


