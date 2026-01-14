use std::os::fd::BorrowedFd;

use libbinder_raw::{commands::{PtrCookieRaw, ReturnVal}, object::reference::ObjectRefLocal, transaction::TransactionKernelManaged};
use yoke::Yokeable;

use crate::packet::Packet;

pub enum ReturnValue<'binder> {
  Transaction((ObjectRefLocal, Packet<'binder>)),
  Acquire(ObjectRefLocal),
  AcquireWeak(ObjectRefLocal),
  Release(ObjectRefLocal),
  ReleaseWeak(ObjectRefLocal),
  Reply(Packet<'binder>),
  TransactionFailed,
  Ok,
  Error(i32),
  SpawnLooper,
  TransactionComplete,
  DeadReply,
  Noop
}

#[derive(Yokeable)]
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
  
  pub fn get_parsed(&self) -> &[ReturnValue<'binder>] {
    &self.parsed
  }
  
  pub fn clear(&mut self) {
    self.parsed.clear();
  }
  
  pub(crate) fn parse(&mut self, read_bytes: usize) {
    let mut current = &self.buffer[..read_bytes];
    const RETVAL_SIZE: usize = size_of::<ReturnVal>();
    while current.len() != 0 {
      let val_tag = ReturnVal::try_from_bytes(current[..RETVAL_SIZE].try_into().unwrap()).unwrap();
      
      let transaction_size = TransactionKernelManaged::bytes_needed();
      let val = match val_tag {
        ReturnVal::Noop => ReturnValue::Noop,
        ReturnVal::Reply => {
          let bytes = &current[RETVAL_SIZE..RETVAL_SIZE+transaction_size];
          let (_, packet) = unsafe { Packet::from_bytes(self.binder_dev, bytes, true) };
          current = &current[transaction_size..];
          ReturnValue::Reply(packet)
        },
        ReturnVal::Transaction => {
          let bytes = &current[RETVAL_SIZE..RETVAL_SIZE+transaction_size];
          let packet = unsafe { Packet::from_bytes(self.binder_dev, bytes, false) };
          current = &current[transaction_size..];
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
        ReturnVal::DeadReply => ReturnValue::DeadReply,
        ReturnVal::DeadBinder => unimplemented!(),
        ReturnVal::Acquire => {
          let ret = PtrCookieRaw::from_raw_bytes(&current[..size_of::<PtrCookieRaw>()]);
          current = &current[size_of::<PtrCookieRaw>()..];
          ReturnValue::Acquire(ObjectRefLocal {
            data: ret.ptr,
            extra_data: ret.cookie
          })
        }
        ReturnVal::Release => {
          let ret = PtrCookieRaw::from_raw_bytes(&current[..size_of::<PtrCookieRaw>()]);
          current = &current[size_of::<PtrCookieRaw>()..];
          ReturnValue::Release(ObjectRefLocal {
            data: ret.ptr,
            extra_data: ret.cookie
          })
        }
        ReturnVal::AcquireWeak => {
          let ret = PtrCookieRaw::from_raw_bytes(&current[..size_of::<PtrCookieRaw>()]);
          current = &current[size_of::<PtrCookieRaw>()..];
          ReturnValue::AcquireWeak(ObjectRefLocal {
            data: ret.ptr,
            extra_data: ret.cookie
          })
        }
        ReturnVal::ReleaseWeak => {
          let ret = PtrCookieRaw::from_raw_bytes(&current[..size_of::<PtrCookieRaw>()]);
          current = &current[size_of::<PtrCookieRaw>()..];
          ReturnValue::ReleaseWeak(ObjectRefLocal {
            data: ret.ptr,
            extra_data: ret.cookie
          })
        }
      };
      
      // Go forward
      current = &current[RETVAL_SIZE..];
      self.parsed.push(val);
    }
  }
  
  // The .0 is cleared and .1 is in unknown state
  // This method mainly useful to convert this buffer into
  // underlying buffers for reuse later under different binder
  // fd.
  pub fn into_buffers(mut self) -> (Vec<ReturnValue<'static>>, Vec<u8>) {
    self.parsed.clear();
    
    // SAFETY: This is safe as the buffer are empty so nothing borrows non static
    // anymore
    let buf = unsafe { std::mem::transmute(self.parsed) };
    (buf, self.buffer)
  }
  
  // .0 is cleared
  // while .1 is left as it is but will be overwritten
  pub fn from_buffers(binder_dev: BorrowedFd<'binder>, mut raw: (Vec<ReturnValue<'static>>, Vec<u8>)) -> Self {
    raw.0.clear();
    
    Self {
      parsed: raw.0,
      buffer: raw.1,
      binder_dev
    }
  }
}



