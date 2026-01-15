use std::{mem, os::fd::BorrowedFd, slice};

use enumflags2::BitFlags;
use libbinder_raw::{object::reference::{ObjectRef, ObjectRefRemote}, transaction::{Transaction, TransactionDataCommon, TransactionFlag, TransactionNotKernelMananged}};

use crate::{formats::WriteFormat, packet::{Packet, writer::Writer}};

#[derive(Clone)]
pub struct PacketBuilder<'binder> {
  pub(super) code: Option<u32>,
  pub(super) binder_dev: BorrowedFd<'binder>,
  pub(super) flags: Option<BitFlags<TransactionFlag>>,
  pub(super) data_buffer: Vec<u8>,
  pub(super) offsets_buffer: Vec<usize>
}

impl<'binder> PacketBuilder<'binder> {
  pub fn new(binder_dev: BorrowedFd<'binder>) -> Self {
    Self {
      code: None,
      flags: None,
      data_buffer: Vec::new(),
      offsets_buffer: Vec::new(),
      binder_dev: binder_dev,
    }
  }
  
  pub fn set_flags(&mut self, flags: BitFlags<TransactionFlag>) -> &mut Self {
    self.flags = Some(flags);
    self
  }
  
  pub fn set_code(&mut self, code: u32) -> &mut Self {
    self.code = Some(code);
    self
  }
  
  pub fn clear(&mut self) {
    self.data_buffer.clear();
    self.offsets_buffer.clear();
    self.flags = None;
    self.code = None;
  }
  
  pub fn get_binder_dev(&self) -> BorrowedFd<'binder> {
    self.binder_dev
  }
  
  // NOTE: This implicitly appends to data written
  // by previous writer
  pub fn writer<'packet, Format: WriteFormat<'packet>>(&'packet mut self, format: Format) -> Writer<'packet, 'binder, Format> {
    Writer::new(self, format)
  }
  
  // After build the builder is 'reset'
  // to state where it starts
  pub fn build(&mut self) -> Packet<'binder> {
    Packet {
      binder_dev: self.binder_dev,
      transaction: Transaction::NotKernelManaged(TransactionNotKernelMananged {
        data: TransactionDataCommon {
          code: self.code.take().expect("code must be given to build a packet"),
          flags: self.flags.take().unwrap_or(BitFlags::empty()),
          target: ObjectRef::Remote(ObjectRefRemote { data_handle: 0, extra_local_data: 0 }),
          data_slice: unsafe { slice::from_raw_parts(self.data_buffer.as_ptr(), self.data_buffer.len()) },
          offsets: unsafe { slice::from_raw_parts(self.offsets_buffer.as_ptr(), self.offsets_buffer.len()) }
        }
      }),
      offset_buffer: mem::replace(&mut self.offsets_buffer, Vec::new()),
      data_buffer: mem::replace(&mut self.data_buffer, Vec::new())
    }
  }
}

