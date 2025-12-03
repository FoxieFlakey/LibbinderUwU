use core::slice;
use std::{mem::ManuallyDrop, os::fd::BorrowedFd};

use bytemuck::{Pod, Zeroable};
use enumflags2::{BitFlags, bitflags};

use crate::{BinderUsize, Command, binder_read_write};

pub enum Transaction<'binder, 'buffer, 'buffer_offsets> {
  NotKernelManaged(TransactionNotKernelMananged<'buffer, 'buffer_offsets>),
  KernelManaged(TransactionKernelManaged<'binder>)
}

pub struct TransactionDataCommon<'buf, 'buf_offsets> {
  pub code: u32,
  pub data_slice: &'buf [u8],
  pub offsets: &'buf_offsets [BinderUsize]
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

pub struct TransactionNotKernelMananged<'buffer, 'buffer_offsets> {
  // A transaction to kernel, targetting this
  // object handle
  pub target: u32,
  pub flags: BitFlags<TransactionFlag>,
  pub data: TransactionDataCommon<'buffer, 'buffer_offsets>
}

impl TransactionNotKernelMananged<'_, '_> {
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

pub struct TransactionKernelManaged<'binder> {
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
  data: ManuallyDrop<TransactionDataCommon<'static, 'static>>
}

impl<'binder> TransactionKernelManaged<'binder> {
  pub fn get_data<'a>(&'a self) -> &'a TransactionDataCommon<'a, 'a> {
    &self.data
  }
  
  // SAFETY: The 'bytes' has to be from kernel from the correct binder_dev
  // and received
  //
  // The 'bytes' alignment can be unaligned, and its fine
  pub unsafe fn from_bytes(&self, binder_dev: BorrowedFd<'binder>, bytes: &[u8]) -> Self {
    if bytes.len() != size_of::<TransactionDataRaw>() {
      panic!("Size of the 'bytes' is not same the size of binder_transaction_data ({} bytes)", size_of::<TransactionDataRaw>());
    }
    
    let temp;
    let aligned = if bytes.as_ptr().addr().is_multiple_of(align_of::<TransactionDataRaw>()) {
        bytes
      } else {
        let mut aligned = Vec::<u8>::new();
        aligned.reserve_exact(bytes.len() + align_of::<TransactionDataRaw>());
        let offset = if aligned.as_ptr().addr().is_power_of_two() {
            0
          } else {
            aligned.as_ptr().addr().next_multiple_of(align_of::<TransactionDataRaw>()) - aligned.as_ptr().addr()
          };
        aligned[offset..].copy_from_slice(bytes);
        temp = aligned;
        &temp[offset..]
      };
    
    assert!(aligned.len() == size_of::<TransactionDataRaw>());
    
    let raw = bytemuck::from_bytes::<TransactionDataRaw>(aligned);
    
    // SAFETY: The buffers data as far as 'static concerned lives longer
    // before the 'static reference gone
    let data_slice: &'static [u8] = unsafe { slice::from_raw_parts(raw.data.ptr.buffer as *mut _, raw.data_size) };
    let offsets: &'static [usize] = unsafe { slice::from_raw_parts(raw.data.ptr.offsets as *mut _, raw.offsets_size) };
    
    Self {
      buffer_ptr: unsafe { raw.data.ptr.buffer },
      extra_data: raw.extra_data,
      object: unsafe { raw.target.binder },
      flags: BitFlags::from_bits(raw.flags).ok().unwrap(),
      data: ManuallyDrop::new(TransactionDataCommon {
        code: raw.code,
        data_slice,
        offsets
      }),
      binder_dev
    }
  }
} 

impl Drop for TransactionKernelManaged<'_> {
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


