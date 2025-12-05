use bytemuck::Pod;

use crate::packet::builder::PacketBuilder;

pub struct Writer<'packet, 'binder> {
  packet: &'packet mut PacketBuilder<'binder>
}

impl<'packet, 'binder> Writer<'packet, 'binder> {
  pub(crate) fn new(packet: &'packet mut PacketBuilder<'binder>) -> Self {
    Self {
      packet
    }
  }
}

macro_rules! impl_primitive {
  ($name:ident, $name_array:ident, $name_array_slice:ident, $type:ty) => {
    pub fn $name_array<const LEN: usize>(&mut self, data: &[$type; LEN]) -> &mut Self {
      self.packet.data_buffer.extend_from_slice(bytemuck::cast_slice(data));
      self
    }
    
    pub fn $name_array_slice(&mut self, data: &[$type]) -> &mut Self {
      self.packet.data_buffer.extend_from_slice(bytemuck::cast_slice(data));
      self
    }
    
    pub fn $name(&mut self, data: $type) -> &mut Self {
      self.packet.data_buffer.extend_from_slice(&data.to_ne_bytes());
      self
    }
  }
}

// The part to handle writes
impl Writer<'_, '_> {
  impl_primitive!(write_u8, write_u8_array, write_u8_slice, u8);
  impl_primitive!(write_u16, write_u16_array, write_u16_slice, u16);
  impl_primitive!(write_u32, write_u32_array, write_u32_slice, u32);
  impl_primitive!(write_u64, write_u64_array, write_u64_slice, u64);
  impl_primitive!(write_u128, write_u128_array, write_u128_slice, u128);
  impl_primitive!(write_usize, write_usize_array, write_usize_slice, usize);
  
  impl_primitive!(write_i8, write_i8_array, write_i8_slice, i8);
  impl_primitive!(write_i16, write_i16_array, write_i16_slice, i16);
  impl_primitive!(write_i32, write_i32_array, write_i32_slice, i32);
  impl_primitive!(write_i64, write_i64_array, write_i64_slice, i64);
  impl_primitive!(write_i128, write_i128_array, write_i128_slice, i128);
  impl_primitive!(write_isize, write_isize_array, write_isize_slice, isize);
  
  impl_primitive!(write_f32, write_f32_array, write_f32_slice, f32);
  impl_primitive!(write_f64, write_f64_array, write_f64_slice, f64);
  
  pub fn write_pod_slice<T: Pod>(&mut self, data: &[T]) -> &mut Self {
    self.packet.data_buffer.extend_from_slice(bytemuck::cast_slice(data));
    self
  }
  
  pub fn write_pod_array<T: Pod, const LEN: usize>(&mut self, data: &[T; LEN]) -> &mut Self {
    self.write_pod_slice(data)
  }
  
  pub fn write_pod<T: Pod>(&mut self, data: &T) -> &mut Self {
    self.packet.data_buffer.extend_from_slice(bytemuck::bytes_of(data));
    self
  }
}

