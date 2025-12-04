use std::{os::fd::BorrowedFd, slice, sync::Arc};

use enumflags2::BitFlags;

use crate::{BinderUsize, Command, ObjectRef, ObjectRefRemote, TransactionDataCommon, binder_read_write, transaction::{BinderOrHandleUnion, BufferStruct, DataUnion, TransactionDataRaw}};

struct KernelBuffer<'binder> {
  binder_dev: BorrowedFd<'binder>,
  buffer_ptr: BinderUsize
}

impl Drop for KernelBuffer<'_> {
  fn drop(&mut self) {
    // There no more reference to the buffer anymore, free the buffer
    let mut commands = Vec::new();
    commands.extend_from_slice(&Command::FreeBuffer.as_bytes());
    commands.extend_from_slice(&self.buffer_ptr.to_ne_bytes());
    
    binder_read_write(self.binder_dev, &commands, &mut []).unwrap();
  }
}

#[derive(Clone)]
pub struct TransactionKernelManaged<'binder> {
  // Cannot specifically make 'static is placeholder mean
  // as long as this struct alive. The getter method turn
  // it into proper borrow to ensure that by time when. Drop
  // runs this is dropped first and safe
  data: TransactionDataCommon<'static, 'static>,
  
  // has to come after the data, as the data refers to
  // the kernel buffer
  _kernel_buf: Arc<KernelBuffer<'binder>>
}

impl<'binder> TransactionKernelManaged<'binder> {
  // Note: We placed fake empty slices, which will be restored
  // don't change the slices. That is to ensure references to
  // buffer don't escape as 'static well just a placeholder
  //
  // To meaning "dynamic lifetime" bound to kernel's buffer that
  // will be free'd later. I could use 'yoke' but it ended up
  // becoming more complicated than necessary
  pub fn with_data_mut<F: FnOnce(&mut TransactionDataCommon<'static, 'static>) -> R, R>(&mut self, func: F) -> R {
    let (buffer_slice_saved, offsets_slice_saved) = (self.data.data_slice, self.data.offsets);
    self.data.data_slice = &[];
    self.data.offsets = &[];
    
    let ret = func(&mut self.data);
    
    self.data.data_slice = buffer_slice_saved;
    self.data.offsets = offsets_slice_saved;
    return ret;
  }
  
  pub fn with_bytes<F: FnOnce(&[u8]) -> R, R>(&self, func: F) -> R {
    let raw = self.as_raw();
    let bytes = bytemuck::bytes_of(&raw);
    func(bytes)
  }
  
  fn as_raw(&self) -> TransactionDataRaw {
    let (target, extra_data) = match &self.data.target {
      ObjectRef::Local(x) => (BinderOrHandleUnion { binder: x.data }, x.extra_data),
      ObjectRef::Remote(x) => (BinderOrHandleUnion { handle: x.data_handle }, 0)
    };
    
    TransactionDataRaw {
      data_size: self.data.data_slice.len(),
      offsets_size: self.data.offsets.len(),
      sender_pid: 0,
      sender_uid: 0,
      flags: self.data.flags.bits(),
      code: self.data.code,
      data: DataUnion {
        ptr: BufferStruct {
          buffer: self.data.data_slice.as_ptr().addr(),
          offsets: self.data.offsets.as_ptr().addr()
        }
      },
      extra_data,
      target
    }
  }
  
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
      _kernel_buf: Arc::new(KernelBuffer {
        buffer_ptr: unsafe { raw.data.ptr.buffer },
        binder_dev
      }),
      data: TransactionDataCommon {
        code: raw.code,
        target: ObjectRef::Remote(ObjectRefRemote {
          data_handle: unsafe { raw.target.handle },
        }),
        flags: BitFlags::from_bits(raw.flags).ok().unwrap(),
        data_slice,
        offsets
      }
    }
  }
} 

