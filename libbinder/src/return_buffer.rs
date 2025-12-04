use std::os::fd::BorrowedFd;

use libbinder_raw::{BYTES_NEEDED_FOR_FROM_BYTES, ObjectRefLocal, ReturnVal};

use crate::packet::Packet;

pub enum ReturnValue<'binder> {
  Transaction(#[expect(unused)] (ObjectRefLocal, Packet<'binder>)),
  Reply(#[expect(unused)] Packet<'binder>),
  TransactionFailed,
  Ok,
  Error(#[expect(unused)] i32),
  SpawnLooper,
  TransactionComplete,
  Noop
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
    self.parsed.clear();
  }
  
  pub(crate) fn parse(&mut self, read_bytes: usize) {
    let mut current = &self.buffer[..read_bytes];
    const RETVAL_SIZE: usize = size_of::<ReturnVal>();
    while current.len() != 0 {
      let val_tag = ReturnVal::try_from_bytes(current[..RETVAL_SIZE].try_into().unwrap()).unwrap();
      
      let val = match val_tag {
        ReturnVal::Noop => ReturnValue::Noop,
        ReturnVal::Reply => {
          let bytes = &current[RETVAL_SIZE..RETVAL_SIZE+BYTES_NEEDED_FOR_FROM_BYTES];
          let (_, packet) = unsafe { Packet::from_bytes(self.binder_dev, bytes, true) };
          current = &current[BYTES_NEEDED_FOR_FROM_BYTES..];
          ReturnValue::Reply(packet)
        },
        ReturnVal::Transaction => {
          let bytes = &current[RETVAL_SIZE..RETVAL_SIZE+BYTES_NEEDED_FOR_FROM_BYTES];
          let packet = unsafe { Packet::from_bytes(self.binder_dev, bytes, false) };
          current = &current[BYTES_NEEDED_FOR_FROM_BYTES..];
          ReturnValue::Transaction((packet.0.unwrap(), packet.1))
        },
        ReturnVal::Error => {
          let err = i32::from_ne_bytes(current[..size_of::<i32>()].try_into().unwrap());
          current = &current[size_of::<i32>()..];
          ReturnValue::Error(err)
        },
        ReturnVal::Failed => ReturnValue::TransactionFailed,
        ReturnVal::Ok => ReturnValue::Ok,
        ReturnVal::SpawnLooper => ReturnValue::SpawnLooper,
        ReturnVal::TransactionComplete => ReturnValue::TransactionComplete,
        _ => panic!()
      };
      
      println!("Ret[{:02}] {:#?}", self.parsed.len(), val_tag);
      
      // Go forward
      current = &current[RETVAL_SIZE..];
      self.parsed.push(val);
    }
  }
}



