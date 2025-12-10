use std::{ffi::CStr, mem};

use libbinder_raw::object::reference::ObjectRef;

use crate::{formats::{InnerWriter, WriteFormat}, packet::builder::PacketBuilder};

pub struct Writer<'packet, 'binder, Format: WriteFormat<'packet>> {
  format: Format,
  result: &'packet mut PacketBuilder<'binder>,
  offsets: Vec<usize>,
}

struct WriterState {
  buffer: Vec<u8>
}

impl InnerWriter<'_> for WriterState {
  fn get_current_offset(&self) -> usize {
    self.buffer.len()
  }
  
  fn write(&mut self, bytes: &[u8]) {
    self.buffer.extend_from_slice(bytes);
  }
  
  fn get_data_buffer_mut(&mut self) -> &mut Vec<u8> {
    &mut self.buffer
  }
}

impl<'packet, Format: WriteFormat<'packet>> Drop for Writer<'packet, '_, Format> {
  fn drop(&mut self) {
    mem::swap(self.format.get_writer().get_data_buffer_mut(), &mut self.result.data_buffer);
    mem::swap(&mut self.result.offsets_buffer, &mut self.offsets);
  }
}

impl<'packet, 'binder, Format: WriteFormat<'packet>> Writer<'packet, 'binder, Format> {
  pub(crate) fn new(packet: &'packet mut PacketBuilder<'binder>, mut format: Format) -> Self {
    let offsets = mem::replace(&mut packet.offsets_buffer, Vec::new());
    format.set_writer(Box::new(WriterState {
      buffer: mem::replace(&mut packet.data_buffer, Vec::new())
    }));
    
    Self {
      result: packet,
      offsets,
      format
    }
  }
  
  pub fn get_current_offset(&mut self) -> usize {
    self.format.get_writer().get_current_offset()
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
impl<'packet, 'binder, Format: WriteFormat<'packet>> Writer<'packet, 'binder, Format> {
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
  
  // The object reference to write has to live as long as the packet itself
  // or if it get sent out, it has to live indefinitely until kernel issues
  // BR_RELEASE and need to ensure its correct reference for correct
  // binder device
  pub unsafe fn write_obj_ref(&mut self, obj_ref: ObjectRef) {
    let offset = self.format.get_writer().get_current_offset();
    if !offset.is_multiple_of(size_of::<u32>()) {
      let bytes_to_align = offset.next_multiple_of(size_of::<u32>()) - offset;
      for _ in 0..bytes_to_align {
        self.write_u8(0);
      }
    }
    let offset = self.format.get_writer().get_current_offset();
    assert!(offset.is_multiple_of(size_of::<u32>()));
    
    self.offsets.push(offset);
    obj_ref.with_raw_bytes(|bytes| {
      self.format.get_writer().write(bytes);
    });
  }
}

