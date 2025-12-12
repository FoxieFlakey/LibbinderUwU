use std::{io, os::fd::BorrowedFd};

use enumflags2::BitFlags;
use libbinder_raw::{object::reference::{ObjectRef, ObjectRefLocal}, transaction::{Transaction, TransactionFlag, TransactionKernelManaged}, types::Type};

use crate::{formats::ReadFormat, packet::{builder::PacketBuilder, reader::Reader}};

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
  
  pub fn get_flags(&self) -> BitFlags<TransactionFlag> {
    self.transaction.get_common().flags
  }
  
  pub fn get_binder_dev(&self) -> BorrowedFd<'binder> {
    self.binder_dev
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
  
  pub(crate) fn get_transaction<'a>(&'a self) -> &'a Transaction<'binder, 'a, 'a> {
    &self.transaction
  }
}


