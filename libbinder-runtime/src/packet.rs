// This is same as packet and related stuffs in libbinder
// but with extended methods, to ensure
// safety of object references stored in it

use std::{ffi::CStr, ops::Deref, os::fd::AsRawFd};

use enumflags2::BitFlags;
use libbinder::{formats::{ReadFormat, SliceReadResult, WriteFormat}, packet::{Packet as PacketUnderlying, builder::PacketBuilder as PacketBuilderUnderlying, reader::Reader, writer::Writer}};

use crate::{Runtime, binder_object::BinderObject};

// This struct has an invariant that all object reference in underlying packet
// belong to the same runtime
pub struct Packet<'runtime, ContextManager: BinderObject<ContextManager>> {
  runtime: &'runtime Runtime<ContextManager>,
  inner: PacketUnderlying<'runtime>
}

pub struct PacketReader<'runtime, 'packet, ContextManager: BinderObject<ContextManager>, Format: ReadFormat<'packet>> {
  inner: Reader<'packet, 'runtime, Format>,
  #[expect(unused)]
  runtime: &'runtime Runtime<ContextManager>
}

// This struct has an invariant that all object reference in underlying packet
// belong to the same runtime
pub struct PacketBuilder<'runtime, ContextManager: BinderObject<ContextManager>> {
  runtime: &'runtime Runtime<ContextManager>,
  inner: PacketBuilderUnderlying<'runtime>
}

pub struct PacketWriter<'runtime, 'packet, ContextManager: BinderObject<ContextManager>, Format: WriteFormat<'packet>> {
  inner: Writer<'packet, 'runtime, Format>,
  #[expect(unused)]
  runtime: &'runtime Runtime<ContextManager>
}

impl<'runtime, ContextManager: BinderObject<ContextManager>> Packet<'runtime, ContextManager> {
  pub(crate) fn new(runtime: &'runtime Runtime<ContextManager>, packet: PacketUnderlying<'runtime> ) -> Self {
    assert!(runtime.shared.binder_dev.as_raw_fd() == packet.get_binder_dev().as_raw_fd(), "attempting to construct packet using packet belonging to different runtime");
    Self {
      runtime,
      inner: packet
    }
  }
  
  pub fn reader<'packet, Format: ReadFormat<'packet>>(&'packet self, format: Format) -> PacketReader<'runtime, 'packet, ContextManager, Format> {
    PacketReader {
      inner: self.inner.reader(format),
      runtime: self.runtime
    }
  }
}

impl<'runtime, ContextManager: BinderObject<ContextManager>> Deref for Packet<'runtime, ContextManager> {
  type Target = PacketUnderlying<'runtime>;
  
  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl<'runtime, ContextManager: BinderObject<ContextManager>> PacketBuilder<'runtime, ContextManager> {
  pub(crate) fn new(runtime: &'runtime Runtime<ContextManager>) -> Self {
    Self {
      runtime,
      inner: PacketBuilderUnderlying::new(runtime.get_binder())
    }
  }
  
  pub fn writer<'packet, Format: WriteFormat<'packet>>(&'packet mut self, format: Format) -> PacketWriter<'runtime, 'packet, ContextManager, Format> {
    PacketWriter {
      inner: self.inner.writer(format),
      runtime: self.runtime
    }
  }
  
  // Additional methods for the &mut borrows
  pub fn set_flags(&mut self, flags: BitFlags<libbinder_raw::transaction::TransactionFlag, u32>) -> &mut Self {
    self.inner.set_flags(flags);
    self
  }
  
  pub fn set_code(&mut self, code: u32) -> &mut Self{
    self.inner.set_code(code);
    self
  }
  
  pub fn clear(&mut self) {
    self.inner.clear();
  }
  
  pub fn build(&mut self) -> Packet<'runtime, ContextManager> {
    let ret = Packet::new(self.runtime, self.inner.build());
    self.clear();
    ret
  }
}

macro_rules! impl_forward {
  ($name:ident, $name_array:ident, $name_array_slice:ident, $type:ty) => {
    pub fn $name_array_slice(&mut self, data: &[$type]) -> &mut Self {
      self.inner.$name_array_slice(data);
      self
    }
    
    pub fn $name_array<const LEN: usize>(&mut self, data: &[$type; LEN]) -> &mut Self {
      self.$name_array_slice(data.as_slice());
      self
    }
    
    pub fn $name(&mut self, data: $type) -> &mut Self {
      self.inner.$name(data);
      self
    }
  }
}

// NOTE: Cannot provide DerefMut due the underlying packet writer might be replaced
// by the user when unwanted
impl<'runtime, 'packet, ContextManager: BinderObject<ContextManager>, Format: WriteFormat<'packet>> PacketWriter<'runtime, 'packet, ContextManager, Format> {
  impl_forward!(write_u8, write_u8_array, write_u8_slice, u8);
  impl_forward!(write_u16, write_u16_array, write_u16_slice, u16);
  impl_forward!(write_u32, write_u32_array, write_u32_slice, u32);
  impl_forward!(write_u64, write_u64_array, write_u64_slice, u64);
  impl_forward!(write_usize, write_usize_array, write_usize_slice, usize);
  
  impl_forward!(write_i8, write_i8_array, write_i8_slice, i8);
  impl_forward!(write_i16, write_i16_array, write_i16_slice, i16);
  impl_forward!(write_i32, write_i32_array, write_i32_slice, i32);
  impl_forward!(write_i64, write_i64_array, write_i64_slice, i64);
  impl_forward!(write_isize, write_isize_array, write_isize_slice, isize);
  
  impl_forward!(write_f32, write_f32_array, write_f32_slice, f32);
  impl_forward!(write_f64, write_f64_array, write_f64_slice, f64);
  impl_forward!(write_str, write_str_array, write_str_slice, &str);
  impl_forward!(write_cstr, write_cstr_array, write_cstr_slice, &CStr);
  impl_forward!(write_bool, write_bool_array, write_bool_slice, bool);
  
  // Additional extension for writer here
}

macro_rules! forward {
  ($name:ident, $type:ty) => {
    pub fn $name(&mut self) -> Result<$type, ()> {
      self.inner.$name()
    }
  };
}

impl<'packet, ContextManager: BinderObject<ContextManager>, Format: ReadFormat<'packet>> PacketReader<'_, 'packet, ContextManager, Format> {
  forward!(read_u8, u8);
  forward!(read_u16, u16);
  forward!(read_u32, u32);
  forward!(read_u64, u64);
  forward!(read_usize, usize);
  
  forward!(read_i8, i8);
  forward!(read_i16, i16);
  forward!(read_i32, i32);
  forward!(read_i64, i64);
  forward!(read_isize, isize);
  
  forward!(read_f32, f32);
  forward!(read_f64, f64);
  forward!(read_str, &'packet str);
  forward!(read_cstr, &'packet CStr);
  forward!(read_bool, bool);
  
  forward!(read_u8_slice, &'packet [u8]);
  forward!(read_u16_slice, SliceReadResult<'packet, u16>);
  forward!(read_u32_slice, SliceReadResult<'packet, u32>);
  forward!(read_u64_slice, SliceReadResult<'packet, u64>);
  forward!(read_usize_slice, SliceReadResult<'packet, usize>);
  
  forward!(read_i8_slice, &'packet [i8]);
  forward!(read_i16_slice, SliceReadResult<'packet, i16>);
  forward!(read_i32_slice, SliceReadResult<'packet, i32>);
  forward!(read_i64_slice, SliceReadResult<'packet, i64>);
  forward!(read_isize_slice, SliceReadResult<'packet, isize>);
  
  forward!(read_f32_slice, SliceReadResult<'packet, f32>);
  forward!(read_f64_slice, SliceReadResult<'packet, f64>);
  
  // Additional extension for reader here
}

