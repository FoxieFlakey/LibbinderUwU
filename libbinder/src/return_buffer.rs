use std::os::fd::BorrowedFd;

use crate::packet::Packet;

pub enum ReturnValue<'binder> {
  Transaction(Packet<'binder>),
  Reply(Packet<'binder>)
}

pub struct ReturnBuffer<'binder> {
  binder_dev: BorrowedFd<'binder>,
  pub(super) buffer: Vec<u8>,
  parsed: Vec<ReturnValue<'binder>>
}

impl<'binder> ReturnBuffer<'binder> {
  pub fn new(binder_dev: BorrowedFd<'binder>, size: usize) -> Self {
    Self {
      buffer: {
        let mut tmp = Vec::new();
        tmp.resize(size, 0);
        tmp
      },
      parsed: Vec::new(),
      binder_dev
    }
  }
  
  pub fn clear(&mut self) {
    self.buffer.clear();
    self.parsed.clear();
  }
  
  pub(crate) fn parse(&mut self) {
  }
}



