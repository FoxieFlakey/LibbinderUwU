use std::{ffi::CStr, marker::PhantomData, mem};

use libbinder_raw::object::reference::ObjectRef;

use crate::{formats::{InnerWriter, WriteFormat}, packet::builder::PacketBuilder};

pub struct Writer<'packet, Format: WriteFormat<'packet>> {
  format: Format,
  offsets: Vec<usize>,
  _phantom: PhantomData<&'packet ()>
}

struct WriterState<'builder> {
  packet: &'builder mut PacketBuilder
}

impl<'builder> InnerWriter<'builder> for WriterState<'builder> {
  fn get_current_offset(&self) -> usize {
    self.packet.data_buffer.len()
  }
  
  fn write(&mut self, bytes: &[u8]) {
    self.packet.data_buffer.extend_from_slice(bytes);
  }
}

impl<'packet, Format: WriteFormat<'packet>> Writer<'packet, Format> {
  pub(crate) fn new(packet: &'packet mut PacketBuilder, mut format: Format) -> Self {
    let offsets = mem::replace(&mut packet.offsets_buffer, Vec::new());
    format.set_writer(Box::new(WriterState {
      packet
    }));
    
    Self {
      _phantom: PhantomData {},
      offsets,
      format
    }
  }
}

macro_rules! impl_forward {
  ($name:ident, $name_array:ident, $name_array_slice:ident, $type:ty) => {
    pub fn $name_array_slice(&mut self, data: &[$type]) -> &mut Self {
      self.format.$name_array_slice(data);
      self
    }
    
    pub fn $name_array<const LEN: usize>(&mut self, data: &[$type; LEN]) -> &mut Self {
      self.$name_array_slice(data.as_slice());
      self
    }
    
    pub fn $name(&mut self, data: $type) -> &mut Self {
      self.format.$name(data);
      self
    }
  }
}

// The part to handle writes
impl<'packet, Format: WriteFormat<'packet>> Writer<'packet, Format> {
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
  
  pub fn write_obj_ref(&mut self, obj_ref: ObjectRef) {
    let offset = self.format.get_writer().get_current_offset();
    self.offsets.push(offset);
    obj_ref.with_raw_bytes(|bytes| {
      self.format.get_writer().write(bytes);
    });
  }
}

