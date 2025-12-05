// This is a library for interfacing with kernel (mainly contains binding to binder)
// with some minimal thingy to ease

use std::os::fd::{AsRawFd, BorrowedFd};

use nix::errno::Errno;

mod object_ref;
mod object;
mod write_read;
mod commands;
mod transaction;

pub use object_ref::{ObjectRefRemote, ObjectRefLocal, ObjectRefFlags, ObjectRef, CONTEXT_MANAGER_REF};
pub use commands::{Command, ReturnVal};
pub use transaction::{TransactionDataCommon, TransactionKernelManaged, TransactionNotKernelMananged, TransactionFlag, Transaction, BYTES_NEEDED_FOR_FROM_BYTES};
pub use write_read::binder_read_write;

// Equivalent to struct binder_version
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
  pub version: i32
}

// Equivalent to struct binder_object_header
#[repr(C)]
#[derive(Clone)]
pub struct ObjectHeader {
  kind: u32
}

// TODO: Make sure binder_uintptr_t is correct? somehow detect BINDER_IPC_32BIT
// it looked like BINDER_IPC_32BIT is for 32-bit userspace, but if im in 32-bit userspace
// then 'usize' is 32-bit so not wrong? but not sure how accurate it is. Maybe 64-bit
// userspace use 32-bit binder for some reason?
pub type BinderUsize = usize;

mod ioctl {
  use nix::{ioctl_readwrite, ioctl_write_ptr};
  use crate::{Version, object_ref::ObjectRefRaw, write_read::ReadWrite};
  
  const BINDER_IOC_MAGIC: u8  = b'b';
  const BINDER_IOC_TYPE_WRITE_READ: u8 = 1;
  const BINDER_IOC_TYPE_VERSION: u8 = 9;
  const BINDER_IOC_SET_CONTEXT_MGR_EXT: u8 = 13;

  ioctl_readwrite!(ioctl_binder_version, BINDER_IOC_MAGIC, BINDER_IOC_TYPE_VERSION, Version);
  ioctl_readwrite!(ioctl_binder_write_read, BINDER_IOC_MAGIC, BINDER_IOC_TYPE_WRITE_READ, ReadWrite);
  ioctl_write_ptr!(ioctl_set_context_mgr_ext, BINDER_IOC_MAGIC, BINDER_IOC_SET_CONTEXT_MGR_EXT, ObjectRefRaw);
}

pub const BINDER_COMPILED_VERSION: Version = Version {
  version: 8
};

pub fn binder_set_context_mgr(fd: BorrowedFd, manager_object: &ObjectRefLocal) -> Result<(), Errno> {
  let mut obj_ref = manager_object.into_raw();
  unsafe { ioctl::ioctl_set_context_mgr_ext(fd.as_raw_fd(), &raw mut obj_ref) }?;
  drop(obj_ref);
  Ok(())
}

pub fn binder_version(fd: BorrowedFd) -> Result<Version, Errno> {
  let mut ver = BINDER_COMPILED_VERSION;
  unsafe { ioctl::ioctl_binder_version(fd.as_raw_fd(), &mut ver) }?;
  Ok(ver)
}


