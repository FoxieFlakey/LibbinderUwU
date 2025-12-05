// Dead simple format, no additional data

use std::ffi::CStr;

use crate::formats::WriteFormat;

pub struct DeadSimpleFormat<'writer> {
  writer: Option<Box<dyn FnMut(&[u8]) + 'writer>>
}

impl<'writer> DeadSimpleFormat<'writer> {
  pub fn new() -> Self {
    Self {
      writer: None
    }
  }
}

impl<'writer> WriteFormat<'writer> for DeadSimpleFormat<'writer> {
  fn set_writer(&mut self, writer: Box<dyn FnMut(&[u8]) + 'writer>) {
    self.writer = Some(writer);
  }
  
  fn get_writer(&mut self) -> &mut dyn FnMut(&[u8]) {
    self.writer.as_mut().unwrap()
  }
  
  fn write_u8(&mut self, data: u8) {
    let writer = self.writer.as_mut().unwrap();
    writer(&data.to_ne_bytes());
  }
  
  fn write_u16(&mut self, data: u16) {
    let writer = self.writer.as_mut().unwrap();
    writer(&data.to_ne_bytes());
  }
  
  fn write_u32(&mut self, data: u32) {
    let writer = self.writer.as_mut().unwrap();
    writer(&data.to_ne_bytes());
  }
  
  fn write_u64(&mut self, data: u64) {
    let writer = self.writer.as_mut().unwrap();
    writer(&data.to_ne_bytes());
  }
  
  fn write_usize(&mut self, data: usize) {
    let writer = self.writer.as_mut().unwrap();
    writer(&data.to_ne_bytes());
  }
  
  fn write_i8(&mut self, data: i8) {
    let writer = self.writer.as_mut().unwrap();
    writer(&data.to_ne_bytes());
  }
  
  fn write_i16(&mut self, data: i16) {
    let writer = self.writer.as_mut().unwrap();
    writer(&data.to_ne_bytes());
  }
  
  fn write_i32(&mut self, data: i32) {
    let writer = self.writer.as_mut().unwrap();
    writer(&data.to_ne_bytes());
  }
  
  fn write_i64(&mut self, data: i64) {
    let writer = self.writer.as_mut().unwrap();
    writer(&data.to_ne_bytes());
  }
  
  fn write_isize(&mut self, data: isize) {
    let writer = self.writer.as_mut().unwrap();
    writer(&data.to_ne_bytes());
  }
  
  fn write_f32(&mut self, data: f32) {
    let writer = self.writer.as_mut().unwrap();
    writer(&data.to_ne_bytes());
  }
  
  fn write_f64(&mut self, data: f64) {
    let writer = self.writer.as_mut().unwrap();
    writer(&data.to_ne_bytes());
  }
  
  fn write_bool(&mut self, data: bool) {
    self.write_u8(data as u8)
  }
  
  fn write_str(&mut self, data: &str) {
    self.write_usize(data.len());
    self.write_u8_slice(data.as_bytes());
  }
  
  fn write_cstr(&mut self, data: &std::ffi::CStr) {
    let writer = self.writer.as_mut().unwrap();
    writer(data.to_bytes_with_nul());
  }
  
  fn write_u8_slice(&mut self, data: &[u8]) {
    self.write_usize(data.len());
    let writer = self.writer.as_mut().unwrap();
    writer(data);
  }
  
  fn write_u16_slice(&mut self, data: &[u16]) {
    self.write_usize(data.len());
    let writer = self.writer.as_mut().unwrap();
    writer(bytemuck::cast_slice(data));
  }
  
  fn write_u32_slice(&mut self, data: &[u32]) {
    self.write_usize(data.len());
    let writer = self.writer.as_mut().unwrap();
    writer(bytemuck::cast_slice(data));
  }
  
  fn write_u64_slice(&mut self, data: &[u64]) {
    self.write_usize(data.len());
    let writer = self.writer.as_mut().unwrap();
    writer(bytemuck::cast_slice(data));
  }
  
  fn write_usize_slice(&mut self, data: &[usize]) {
    self.write_usize(data.len());
    let writer = self.writer.as_mut().unwrap();
    writer(bytemuck::cast_slice(data));
  }
  
  fn write_i8_slice(&mut self, data: &[i8]) {
    self.write_usize(data.len());
    let writer = self.writer.as_mut().unwrap();
    writer(bytemuck::cast_slice(data));
  }
  
  fn write_i16_slice(&mut self, data: &[i16]) {
    self.write_usize(data.len());
    let writer = self.writer.as_mut().unwrap();
    writer(bytemuck::cast_slice(data));
  }
  
  fn write_i32_slice(&mut self, data: &[i32]) {
    self.write_usize(data.len());
    let writer = self.writer.as_mut().unwrap();
    writer(bytemuck::cast_slice(data));
  }
  
  fn write_i64_slice(&mut self, data: &[i64]) {
    self.write_usize(data.len());
    let writer = self.writer.as_mut().unwrap();
    writer(bytemuck::cast_slice(data));
  }
  
  fn write_isize_slice(&mut self, data: &[isize]) {
    self.write_usize(data.len());
    let writer = self.writer.as_mut().unwrap();
    writer(bytemuck::cast_slice(data));
  }
  
  fn write_f32_slice(&mut self, data: &[f32]) {
    self.write_usize(data.len());
    let writer = self.writer.as_mut().unwrap();
    writer(bytemuck::cast_slice(data));
  }
  
  fn write_f64_slice(&mut self, data: &[f64]) {
    self.write_usize(data.len());
    let writer = self.writer.as_mut().unwrap();
    writer(bytemuck::cast_slice(data));
  }
  
  fn write_str_slice(&mut self, data: &[&str]) {
    self.write_usize(data.len());
    for string in data {
      self.write_str(string);
    }
  }
  
  fn write_cstr_slice(&mut self, data: &[&CStr]) {
    self.write_usize(data.len());
    for string in data {
      self.write_cstr(string);
    }
  }
  
  fn write_bool_slice(&mut self, data: &[bool]) {
    self.write_usize(data.len());
    let writer = self.writer.as_mut().unwrap();
    writer(bytemuck::cast_slice(data));
  }
}


