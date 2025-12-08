#![feature(never_type)]
#![feature(alloc_layout_extra)]

use std::os::fd::BorrowedFd;

use libbinder_raw::types::reference::ObjectRef as ObjectRefRaw;

pub mod packet;
pub mod command_buffer;
pub mod return_buffer;
pub mod formats;

// This structure has invariance of
// the origin of reference is exactly
// same for the object reference. and
// outside crate cannot break this invariance
// without using unsafe function
pub struct ObjectRef<'binder> {
  binder: BorrowedFd<'binder>,
  reference: ObjectRefRaw
}

impl<'binder> ObjectRef<'binder> {
  pub(crate) fn new(binder: BorrowedFd<'binder>, reference: ObjectRefRaw) -> Self {
    Self {
      binder,
      reference
    }
  }
  
  pub fn get_binder(&self) -> BorrowedFd<'binder> {
    self.binder
  }
  
  pub fn get_reference(&self) -> &ObjectRefRaw {
    &self.reference
  }
}

