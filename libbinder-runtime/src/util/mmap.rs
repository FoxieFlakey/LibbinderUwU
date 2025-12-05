use std::{ffi::CString, mem, ops::Range, os::fd::{AsRawFd, BorrowedFd}, ptr};

use enumflags2::{bitflags, BitFlags};
use nix::{libc::{MAP_ANON, MAP_FAILED, MAP_FIXED, MAP_POPULATE, MAP_PRIVATE, MAP_SHARED, PR_SET_VMA, PR_SET_VMA_ANON_NAME, PROT_EXEC, PROT_READ, PROT_WRITE, c_int, c_long, c_ulong, mmap, munmap, prctl}, Error};
use sync_ptr::SyncMutPtr;

pub struct MmapRegion {
  // For provenance stuffs
  base_addr: SyncMutPtr<u8>,
  // Its page address (essentially address divided by page size)
  memory: Range<usize>
}

#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy)]
pub enum Protection {
  Execute,
  Read,
  Write
}

#[derive(Debug, Clone, Copy)]
pub enum MmapError {
  MmapError(#[expect(unused)] nix::Error)
}

impl Drop for MmapRegion {
  fn drop(&mut self) {
    let mem = self.get_range_mut().start;
    // SAFETY: This is unused now
    let ret = unsafe { munmap(mem.cast(), self.get_bytes_size()) };
    assert!(ret == 0);
  }
}

#[derive(Debug, Clone, Copy)]
pub struct MemorySpan {
  pub addr: Option<usize>,
  pub nr_pages: usize
}

impl MmapRegion {
  pub fn get_pg_count(&self) -> usize {
    self.memory.end - self.memory.start
  }
  
  pub fn get_range(&self) -> Range<*const u8> {
    Range {
      start: self.get_range_mut().start.cast_const(),
      end: self.get_range_mut().end.cast_const(),
    }
  }
  
  pub fn get_range_mut(&self) -> Range<*mut u8> {
    let offset_from_start = (self.memory.end - self.memory.start) * page_size::get();
    let start = self.base_addr.with_addr(self.memory.start * page_size::get());
    let end = start.wrapping_byte_add(offset_from_start);
    
    Range {
      start,
      end
    }
  }
  
  pub fn get_bytes_size(&self) -> usize {
    self.get_pg_count() * page_size::get()
  }
  
  pub fn new(span: MemorySpan, flags: BitFlags<Protection>, is_shared: bool) -> Result<Self, MmapError> {
    Self::new_impl(span, flags, is_shared, false, None)
  }
  
  // NOTE: Lifetime is not described here but
  // the resulting mapping is tied to the fd
  // the kernel automatically keep fd lives
  // longer even the fd number is not used anymore
  pub fn new_map_from_fd(span: MemorySpan, flags: BitFlags<Protection>, is_shared: bool, do_prefault: bool, fd: BorrowedFd, offset: usize) -> Result<Self, MmapError> {
    Self::new_impl(span, flags, is_shared, do_prefault, Some((fd, offset)))
  }
  
  fn new_impl(span: MemorySpan, flags: BitFlags<Protection>, is_shared: bool, do_prefault: bool, fd_and_offset: Option<(BorrowedFd, usize)>) -> Result<Self, MmapError> {
    let mut map_flags = 0;
    if do_prefault {
      map_flags |= MAP_POPULATE;
    }
    
    if fd_and_offset.is_none() {
      map_flags |= MAP_ANON;
    }
    
    if span.addr.is_some() {
      map_flags |= MAP_FIXED;
    }
    
    if is_shared {
      map_flags |= MAP_SHARED;
    } else {
      map_flags |= MAP_PRIVATE;
    }
    
    let mapped_pages = unsafe {
      mmap(
        span.addr.map(|x| ptr::without_provenance_mut(x)).unwrap_or(ptr::null_mut()),
        span.nr_pages * page_size::get(),
        into_mmap_flags(flags),
        map_flags,
        fd_and_offset
          .map(|x| x.0.as_raw_fd())
          .unwrap_or(-1),
        fd_and_offset
          .map(|x| x.1)
          .map(i64::try_from)
          .map(|x| Result::ok(x).unwrap())
          .unwrap_or(0),
      )
    };
    
    if mapped_pages == MAP_FAILED {
      return Err(MmapError::MmapError(Error::last()));
    }
    
    assert!(mapped_pages.addr().is_multiple_of(page_size::get()));
    
    let start = mapped_pages.expose_provenance() / page_size::get();
    let end = start + span.nr_pages;
    
    Ok(
      Self {
        base_addr: SyncMutPtr::new(mapped_pages.cast()),
        memory: Range { start, end }
      }
    )
  }
  
  pub fn leak(self) -> Range<*mut u8> {
    let range = self.get_range_mut();
    mem::forget(self);
    range
  } 
  
  pub fn set_name(&self, str: &str) {
    let str = CString::new(str).unwrap();
    let ret = unsafe { prctl(PR_SET_VMA, PR_SET_VMA_ANON_NAME as c_long, self.get_range().start.addr() as c_ulong, self.get_bytes_size() as c_ulong, str.as_ptr()) };
    if ret == -1 {
      panic!("Error setting name on VMA: {}", Error::last().desc());
    }
    drop(str);
  }
  
  // # Safety
  // 'range' start and end must page aligned the size is
  // implicitly page aligned and non empty and valid memory
  // mapped from mmap
  pub unsafe fn from_raw_ptr_range(range: Range<*mut u8>) -> Self {
    assert!(!range.is_empty());
    assert!(range.start.addr().is_multiple_of(page_size::get()));
    assert!(range.end.addr().is_multiple_of(page_size::get()));
    assert!((range.end.addr() - range.start.addr()).is_multiple_of(page_size::get()));
    
    MmapRegion {
      base_addr: SyncMutPtr::new(range.start),
      memory: Range {
        start: range.start.addr() / page_size::get(),
        end: range.end.addr() / page_size::get()
      }
    }
  }
}

fn into_mmap_flags(flags: BitFlags<Protection>) -> c_int {
  if flags.is_empty() {
    return 0 as c_int;
  }
  
  let mut res = 0;
  if flags.contains(Protection::Execute) {
    res |= PROT_EXEC;
  }
  
  if flags.contains(Protection::Read) {
    res |= PROT_READ;
  }
  
  if flags.contains(Protection::Write) {
    res |= PROT_WRITE;
  }
  res
}

