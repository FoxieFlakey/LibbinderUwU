use std::{mem, os::fd::BorrowedFd, slice};

use enumflags2::BitFlags;
use libbinder_raw::{ObjectRef, Transaction, TransactionDataCommon, TransactionFlag, TransactionNotKernelMananged};

use crate::packet::Packet;

pub struct PacketBuilder<'binder> {
  binder_dev: Option<BorrowedFd<'binder>>,
  code: Option<u32>,
  flags: Option<BitFlags<TransactionFlag>>,
  target: Option<ObjectRef>,
  data_buffer: Vec<u8>,
  offsets_buffer: Vec<usize>
}

impl<'binder> PacketBuilder<'binder> {
  pub fn new() -> Self {
    Self {
      binder_dev: None,
      code: None,
      flags: None,
      target: None,
      data_buffer: Vec::new(),
      offsets_buffer: Vec::new()
    }
  }
  
  pub fn set_flags(&mut self, flags: BitFlags<TransactionFlag>) -> &mut Self {
    self.flags = Some(flags);
    self
  }
  
  pub fn set_target(&mut self, target: ObjectRef) -> &mut Self {
    self.target = Some(target);
    self
  }
  
  pub fn set_code(&mut self, code: u32) -> &mut Self {
    self.code = Some(code);
    self
  }
  
  pub fn set_binder_dev(&mut self, binder_dev: BorrowedFd<'binder>) -> &mut Self {
    self.binder_dev = Some(binder_dev);
    self
  }
  
  // After build the builder is 'reset'
  // to state where it starts
  pub fn build(&mut self) -> Packet<'binder> {
    Packet {
      binder_dev: self.binder_dev.take().expect("binder_dev must be given to build a packet"),
      transaction: Transaction::NotKernelManaged(TransactionNotKernelMananged {
        data: TransactionDataCommon {
          code: self.code.take().expect("code must be given to build a packet"),
          flags: self.flags.take().unwrap_or(BitFlags::empty()),
          target: self.target.take().expect("target must be given to build a packet"),
          data_slice: unsafe { slice::from_raw_parts(self.data_buffer.as_ptr(), self.data_buffer.len()) },
          offsets: unsafe { slice::from_raw_parts(self.offsets_buffer.as_ptr(), self.offsets_buffer.len()) }
        }
      }),
      offset_buffer: mem::replace(&mut self.offsets_buffer, Vec::new()),
      data_buffer: mem::replace(&mut self.data_buffer, Vec::new())
    }
  }
}

