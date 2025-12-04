use std::{mem::ManuallyDrop, os::fd::BorrowedFd, slice};

use enumflags2::BitFlags;

use crate::{BinderUsize, Command, ObjectRef, ObjectRefRemote, TransactionDataCommon, binder_read_write, transaction::TransactionDataRaw};

pub struct TransactionKernelManaged<'binder> {
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
  // and the bytes assumed to be from BR_TRANSACTION/BR_REPLY
  //
  // The 'bytes' alignment can be unaligned, and its fine
  pub unsafe fn from_bytes(binder_dev: BorrowedFd<'binder>, bytes: &[u8]) -> Self {
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
      data: ManuallyDrop::new(TransactionDataCommon {
        code: raw.code,
        target: ObjectRef::Remote(ObjectRefRemote {
          data_handle: unsafe { raw.target.handle },
        }),
        flags: BitFlags::from_bits(raw.flags).ok().unwrap(),
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
