use std::{cell::RefCell, os::fd::BorrowedFd};

use enumflags2::BitFlags;
use libbinder_raw::{Command, ObjectRef, Transaction, TransactionFlag, binder_read_write};

use crate::hexdump;

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
  
  #[expect(unused)]
  data_buffer: Vec<u8>,
  #[expect(unused)]
  offset_buffer: Vec<usize>
}

impl<'binder> Packet<'binder> {
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
  
  #[expect(unused)]
  pub fn set_target(&mut self, target: ObjectRef) {
    self.transaction.with_common_mut(|common| {
      common.target = target;
    });
  }
  
  pub fn send(&self) -> Packet<'binder> {
    let target = self.transaction.get_common().target.clone();
    if matches!(target, ObjectRef::Local(_)) {
      todo!("Handle local transaction");
    }
    
    thread_local! {
      static BUFFER: RefCell<(Vec<u8>, Vec<u8>)> = RefCell::new((Vec::new(), Vec::new()));
    }
    
    BUFFER.with_borrow_mut(|(write_buf, read_buf)| {
      write_buf.clear();
      read_buf.clear();
      read_buf.resize(4000, 0);
      
      // Submit to binder kernel driver
      // for remote transaction
      write_buf.extend_from_slice(&Command::SendTransaction.as_bytes());
      self.transaction.with_bytes(|bytes| write_buf.extend_from_slice(bytes));
      
      let (sent, received) = binder_read_write(self.binder_dev, write_buf, read_buf).unwrap();
      println!("Sent {sent} bytes, received {received} bytes");
      hexdump(&read_buf[..received]);
    });
    
    todo!();
  }
}


