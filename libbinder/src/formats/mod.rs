// Contains various different format for data inside the packet's data buffer
// each write, must be independent that mean do not write header/footer

use std::ffi::CStr;

pub mod dead_simple;

pub enum SliceReadResult<'reader, T> {
  // Incase the data is aligned
  Borrowed(&'reader T),
  
  // Incase the data is not aligned
  // so copy is needed
  Owned(Box<[T]>)
}

pub trait InnerReader<'reader>: 'reader {
  fn clone_reader(&self) -> Box<dyn InnerReader<'reader>>;
  fn peek(&self, size: usize, offset: usize) -> Result<&'reader [u8], ()>;
  fn read(&mut self, size: usize) -> Result<&'reader [u8], ()>;
}

pub trait ReadFormat<'reader>: Clone {
  // Set a reader to use
  // First usize is number of bytes to read
  //
  // Second Option<usize> mean offset of starting for 'peek' reading
  // it does not advance the reader. Mainly used so can compute the length
  // for C string and alike which don't have size written
  fn set_reader(&mut self, reader: Box<dyn InnerReader<'reader>>);
  fn get_reader(&mut self) -> &mut Box<dyn InnerReader<'reader>>;
  
  fn read_u8(&mut self) -> Result<u8, ()>;
  fn read_u16(&mut self) -> Result<u16, ()>;
  fn read_u32(&mut self) -> Result<u32, ()>;
  fn read_u64(&mut self) -> Result<u64, ()>;
  fn read_usize(&mut self) -> Result<usize, ()>;
  
  fn read_i8(&mut self) -> Result<i8, ()>;
  fn read_i16(&mut self) -> Result<i16, ()>;
  fn read_i32(&mut self) -> Result<i32, ()>;
  fn read_i64(&mut self) -> Result<i64, ()>;
  fn read_isize(&mut self) -> Result<isize, ()>;
  
  fn read_f32(&mut self) -> Result<f32, ()>;
  fn read_f64(&mut self) -> Result<f64, ()>;
  fn read_str(&mut self) -> Result<&'reader str, ()>;
  fn read_cstr(&mut self) -> Result<&'reader CStr, ()>;
  fn read_bool(&mut self) -> Result<bool, ()>;
  
  fn read_u8_slice(&mut self) -> Result<&'reader [u8], ()>;
  fn read_u16_slice(&mut self) -> Result<SliceReadResult<'reader, u16>, ()>;
  fn read_u32_slice(&mut self) -> Result<SliceReadResult<'reader, u32>, ()>;
  fn read_u64_slice(&mut self) -> Result<SliceReadResult<'reader, u64>, ()>;
  fn read_usize_slice(&mut self) -> Result<SliceReadResult<'reader, usize>, ()>;
  
  fn read_i8_slice(&mut self) -> Result<&'reader [i8], ()>;
  fn read_i16_slice(&mut self) -> Result<SliceReadResult<'reader, i16>, ()>;
  fn read_i32_slice(&mut self) -> Result<SliceReadResult<'reader, i32>, ()>;
  fn read_i64_slice(&mut self) -> Result<SliceReadResult<'reader, i64>, ()>;
  fn read_isize_slice(&mut self) -> Result<SliceReadResult<'reader, isize>, ()>;
  
  fn read_f32_slice(&mut self) -> Result<SliceReadResult<'reader, f32>, ()>;
  fn read_f64_slice(&mut self) -> Result<SliceReadResult<'reader, f64>, ()>;
  
  // Result Vec for these two will not be cleared, it is appened to it
  // BUT: on error, only partially is pushed into the result
  fn read_str_slice(&mut self, result: &mut Vec<&'reader str>) -> Result<(), ()>;
  fn read_cstr_slice(&mut self, result: &mut Vec<&'reader CStr>) -> Result<(), ()>;
  fn read_bool_slice(&mut self) -> Result<&'reader [bool], ()>;
}

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

