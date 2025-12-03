use std::{mem::ManuallyDrop, os::fd::BorrowedFd};

use bytemuck::{Pod, Zeroable};
use enumflags2::{BitFlags, bitflags};

use crate::{BinderUsize, Command, binder_read_write};

pub struct TransactionDataCommon<'buf> {
  pub code: u32,
  pub data_slice: &'buf [u8],
  pub offsets: &'buf [BinderUsize]
}

#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy)]
pub enum TransactionFlag {
  OneWay = 0x01,
  RootObject = 0x04,
  StatusCode = 0x08,
  AcceptFds = 0x10,
  ClearBuffer = 0x20,
  UpdateTransaction = 0x40
}

pub struct TransactionToKernel<'buffer> {
  // A transaction to kernel, targetting this
  // object handle
  pub target: u32,
  pub flags: BitFlags<TransactionFlag>,
  pub data: TransactionDataCommon<'buffer>
}

impl TransactionToKernel<'_> {
  pub fn with_bytes<R, F: FnOnce(&[u8]) -> R>(&self, func: F) -> R {
    let raw = self.as_raw();
    let bytes = bytemuck::bytes_of(&raw);
    func(bytes)
  }
  
  fn as_raw(&self) -> TransactionDataRaw {
    TransactionDataRaw {
      target: BinderOrHandleUnion {
        handle: self.target
      },
      data_size: 0,
      offsets_size: 0,
      sender_pid: 0,
      sender_uid: 0,
      extra_data: 0,
      flags: self.flags.bits(),
      code: self.data.code,
      data: DataUnion {
        ptr: BufferStruct {
          buffer: 0, offsets: 0
        }
      }
    }
  }
}

pub struct TransactionFromKernel<'binder> {
  // A transaction from kernel targetting this object
  pub object: usize,
  pub flags: BitFlags<TransactionFlag>,
  
  // Extra data associated with the object
  pub extra_data: usize,
  
  // Used by drop code
  binder_dev: BorrowedFd<'binder>,
  buffer_ptr: BinderUsize,
  
  // Cannot specifically make 'static is placeholder mean
  // as long as this struct alive. The getter method turn
  // it into proper borrow to ensure that by time when. Drop
  // runs this is dropped first and safe
  data: ManuallyDrop<TransactionDataCommon<'static>>
}

impl TransactionFromKernel<'_> {
  pub fn get_data<'a>(&'a self) -> &'a TransactionDataCommon<'a> {
    &self.data
  }
} 

impl Drop for TransactionFromKernel<'_> {
  fn drop(&mut self) {
    // SAFETY: 'static reference to the data cannot escape
    unsafe { ManuallyDrop::drop(&mut self.data) };
    
    // There no more reference to the buffer anymore, free the buffer
    let mut commands = Vec::new();
    commands.extend_from_slice(&Command::FreeBuffer.as_bytes());
    commands.extend_from_slice(&self.buffer_ptr.to_ne_bytes());
    
    binder_read_write(self.binder_dev, &commands, &mut []).unwrap();
  }
}

// Union in binder_transaction_data
#[repr(C)]
#[derive(Clone, Copy, Zeroable)]
union BinderOrHandleUnion {
  binder: BinderUsize,
  handle: u32
}

unsafe impl Pod for BinderOrHandleUnion {}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BufferStruct {
  buffer: BinderUsize,
  offsets: BinderUsize
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable)]
union DataUnion {
  ptr: BufferStruct,
  _unused: [u8; 8]
}

unsafe impl Pod for DataUnion {}

// Equivalent to struct binder_transaction_data
#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub(crate) struct TransactionDataRaw {
  target: BinderOrHandleUnion,
  extra_data: usize,
  code: u32,
  flags: u32,
  sender_pid: nix::libc::pid_t,
  sender_uid: nix::libc::uid_t,
  data_size: BinderUsize,
  offsets_size: BinderUsize,
  data: DataUnion
}


