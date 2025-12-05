// Contains various different format for data inside the packet's data buffer
// each write, must be independent that mean do not write header/footer

use std::ffi::CStr;

pub mod dead_simple;

pub trait WriteFormat<'writer> {
  // Set a writer to use
  fn set_writer(&mut self, writer: Box<dyn FnMut(&[u8]) + 'writer>);
  fn get_writer(&mut self) -> &mut dyn FnMut(&[u8]);
  
  fn write_u8(&mut self, data: u8);
  fn write_u16(&mut self, data: u16);
  fn write_u32(&mut self, data: u32);
  fn write_u64(&mut self, data: u64);
  fn write_usize(&mut self, data: usize);
  
  fn write_i8(&mut self, data: i8);
  fn write_i16(&mut self, data: i16);
  fn write_i32(&mut self, data: i32);
  fn write_i64(&mut self, data: i64);
  fn write_isize(&mut self, data: isize);
  
  fn write_f32(&mut self, data: f32);
  fn write_f64(&mut self, data: f64);
  fn write_str(&mut self, data: &str);
  fn write_cstr(&mut self, data: &CStr);
  fn write_bool(&mut self, data: bool);
  
  fn write_u8_slice(&mut self, data: &[u8]);
  fn write_u16_slice(&mut self, data: &[u16]);
  fn write_u32_slice(&mut self, data: &[u32]);
  fn write_u64_slice(&mut self, data: &[u64]);
  fn write_usize_slice(&mut self, data: &[usize]);
  
  fn write_i8_slice(&mut self, data: &[i8]);
  fn write_i16_slice(&mut self, data: &[i16]);
  fn write_i32_slice(&mut self, data: &[i32]);
  fn write_i64_slice(&mut self, data: &[i64]);
  fn write_isize_slice(&mut self, data: &[isize]);
  
  fn write_f32_slice(&mut self, data: &[f32]);
  fn write_f64_slice(&mut self, data: &[f64]);
  fn write_str_slice(&mut self, data: &[&str]);
  fn write_cstr_slice(&mut self, data: &[&CStr]);
  fn write_bool_slice(&mut self, data: &[bool]);
}

