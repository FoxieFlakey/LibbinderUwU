use nix::{errno::Errno, libc};

pub struct OwnedMmap {
  pub ptr: *mut u8,
  pub len: usize
}

unsafe impl Sync for OwnedMmap {}
unsafe impl Send for OwnedMmap {}

impl Drop for OwnedMmap {
  fn drop(&mut self) {
    let ret = unsafe { libc::munmap(self.ptr.cast(), self.len) };
    assert!(ret == 0, "Error munmapping memory: {}", Errno::last().desc());
  }
}

