// Dead simple format, no additional data

use std::{alloc::{self, Layout}, ffi::CStr, slice};

use bytemuck::PodCastError;

use crate::formats::{InnerReader, InnerWriter, ReadFormat, SliceReadResult, WriteFormat};

pub struct DeadSimpleFormat<'writer> {
  writer: Option<Box<dyn InnerWriter<'writer> + 'writer>>
}

impl<'writer> DeadSimpleFormat<'writer> {
  pub fn new() -> Self {
    Self {
      writer: None
    }
  }
}

impl<'writer> WriteFormat<'writer> for DeadSimpleFormat<'writer> {
  fn set_writer(&mut self, writer: Box<dyn InnerWriter<'writer> + 'writer>) {
    self.writer = Some(writer);
  }
  
  fn get_writer_mut(&mut self) -> &mut Box<dyn InnerWriter<'writer> + 'writer> {
    self.writer.as_mut().unwrap()
  }
  
  fn get_writer(&self) -> &Box<dyn InnerWriter<'writer> + 'writer> {
    self.writer.as_ref().unwrap()
  }
  
  fn write_u8(&mut self, data: u8) {
    self.get_writer_mut().write(&data.to_ne_bytes());
  }
  
  fn write_u16(&mut self, data: u16) {
    self.get_writer_mut().write(&data.to_ne_bytes());
  }
  
  fn write_u32(&mut self, data: u32) {
    self.get_writer_mut().write(&data.to_ne_bytes());
  }
  
  fn write_u64(&mut self, data: u64) {
    self.get_writer_mut().write(&data.to_ne_bytes());
  }
  
  fn write_usize(&mut self, data: usize) {
    self.get_writer_mut().write(&data.to_ne_bytes());
  }
  
  fn write_i8(&mut self, data: i8) {
    self.get_writer_mut().write(&data.to_ne_bytes());
  }
  
  fn write_i16(&mut self, data: i16) {
    self.get_writer_mut().write(&data.to_ne_bytes());
  }
  
  fn write_i32(&mut self, data: i32) {
    self.get_writer_mut().write(&data.to_ne_bytes());
  }
  
  fn write_i64(&mut self, data: i64) {
    self.get_writer_mut().write(&data.to_ne_bytes());
  }
  
  fn write_isize(&mut self, data: isize) {
    self.get_writer_mut().write(&data.to_ne_bytes());
  }
  
  fn write_f32(&mut self, data: f32) {
    self.get_writer_mut().write(&data.to_ne_bytes());
  }
  
  fn write_f64(&mut self, data: f64) {
    self.get_writer_mut().write(&data.to_ne_bytes());
  }
  
  fn write_bool(&mut self, data: bool) {
    self.write_u8(data as u8)
  }
  
  fn write_str(&mut self, data: &str) {
    self.write_u8_slice(data.as_bytes());
  }
  
  fn write_cstr(&mut self, data: &std::ffi::CStr) {
    self.get_writer_mut().write(data.to_bytes_with_nul());
  }
  
  fn write_u8_slice(&mut self, data: &[u8]) {
    self.write_usize(data.len());
    self.get_writer_mut().write(data);
  }
  
  fn write_u16_slice(&mut self, data: &[u16]) {
    self.write_usize(data.len());
    self.get_writer_mut().write(bytemuck::cast_slice(data));
  }
  
  fn write_u32_slice(&mut self, data: &[u32]) {
    self.write_usize(data.len());
    self.get_writer_mut().write(bytemuck::cast_slice(data));
  }
  
  fn write_u64_slice(&mut self, data: &[u64]) {
    self.write_usize(data.len());
    self.get_writer_mut().write(bytemuck::cast_slice(data));
  }
  
  fn write_usize_slice(&mut self, data: &[usize]) {
    self.write_usize(data.len());
    self.get_writer_mut().write(bytemuck::cast_slice(data));
  }
  
  fn write_i8_slice(&mut self, data: &[i8]) {
    self.write_usize(data.len());
    self.get_writer_mut().write(bytemuck::cast_slice(data));
  }
  
  fn write_i16_slice(&mut self, data: &[i16]) {
    self.write_usize(data.len());
    self.get_writer_mut().write(bytemuck::cast_slice(data));
  }
  
  fn write_i32_slice(&mut self, data: &[i32]) {
    self.write_usize(data.len());
    self.get_writer_mut().write(bytemuck::cast_slice(data));
  }
  
  fn write_i64_slice(&mut self, data: &[i64]) {
    self.write_usize(data.len());
    self.get_writer_mut().write(bytemuck::cast_slice(data));
  }
  
  fn write_isize_slice(&mut self, data: &[isize]) {
    self.write_usize(data.len());
    self.get_writer_mut().write(bytemuck::cast_slice(data));
  }
  
  fn write_f32_slice(&mut self, data: &[f32]) {
    self.write_usize(data.len());
    self.get_writer_mut().write(bytemuck::cast_slice(data));
  }
  
  fn write_f64_slice(&mut self, data: &[f64]) {
    self.write_usize(data.len());
    self.get_writer_mut().write(bytemuck::cast_slice(data));
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
    self.get_writer_mut().write(bytemuck::cast_slice(data));
  }
}

pub struct DeadSimpleFormatReader<'reader> {
  reader: Option<Box<dyn InnerReader<'reader>>>
}

impl DeadSimpleFormatReader<'_> {
  pub fn new() -> Self {
    Self {
      reader: None
    }
  }
}

impl Clone for DeadSimpleFormatReader<'_> {
  fn clone(&self) -> Self {
    Self {
      reader: self.reader.as_ref().map(|x| x.clone_reader())
    }
  }
}

macro_rules! impl_slice {
  ($name:ident, $type:ty) => {
    fn $name(&mut self) -> Result<SliceReadResult<'reader, $type>, ()> {
      let length = self.read_usize()?;
      let bytes = self.get_reader_mut().read(length * size_of::<$type>())?;
      // Ensure that reader actually read all byte necessary
      assert!(bytes.len() == length * size_of::<$type>());
      Ok(
        bytemuck::try_from_bytes::<$type>(bytes)
          .map(SliceReadResult::Borrowed)
          .unwrap_or_else(|e| {
            // Unable to do cast, alignment might be wrong
            // copy it to aligned space
            
            match e {
              PodCastError::TargetAlignmentGreaterAndInputNotAligned => (),
              _ => panic!("unknown case")
            }
            
            if length == 0 {
              return SliceReadResult::Owned(Box::new([]));
            }
            
            // SAFETY: The layout is non zero in size
            let mem = unsafe { alloc::alloc(
                Layout::new::<$type>()
                  .repeat(length)
                  .unwrap()
                  .0
              ) };
            
            // SAFETY: Just allocated the memory
            unsafe { mem.copy_from_nonoverlapping(bytes.as_ptr(), length * size_of::<$type>()); };
            
            // SAFETY: Written valid data of T previously
            let slice = unsafe { slice::from_raw_parts_mut(mem.cast::<$type>(), length) };
            
            SliceReadResult::Owned(unsafe { Box::from_raw(slice) })
          })
      )
    }
  };
}

impl<'reader> ReadFormat<'reader> for DeadSimpleFormatReader<'reader> {
  fn get_reader_mut(&mut self) -> &mut Box<dyn InnerReader<'reader>> {
    self.reader.as_mut().unwrap()
  }
  
  fn set_reader(&mut self, reader: Box<dyn InnerReader<'reader>>) {
    self.reader = Some(reader);
  }
  
  fn get_reader(&self) -> &Box<dyn InnerReader<'reader>> {
    self.reader.as_ref().unwrap()
  }
  
  fn read_u8(&mut self) -> Result<u8, ()> {
    self.get_reader_mut().read(1)
      .map(|x| x[0])
  }
  
  fn read_u16(&mut self) -> Result<u16, ()> {
    self.get_reader_mut().read(2)
      .map(|x| u16::from_ne_bytes(x.try_into().unwrap()))
  }
  
  fn read_u32(&mut self) -> Result<u32, ()> {
    self.get_reader_mut().read(4)
      .map(|x| u32::from_ne_bytes(x.try_into().unwrap()))
  }
  
  fn read_u64(&mut self) -> Result<u64, ()> {
    self.get_reader_mut().read(8)
      .map(|x| u64::from_ne_bytes(x.try_into().unwrap()))
  }
  
  fn read_usize(&mut self) -> Result<usize, ()> {
    self.get_reader_mut().read(size_of::<usize>())
      .map(|x| usize::from_ne_bytes(x.try_into().unwrap()))
  }
  
  fn read_i8(&mut self) -> Result<i8, ()> {
    self.get_reader_mut().read(1)
      .map(|x| i8::from_ne_bytes(x.try_into().unwrap()))
  }
  
  fn read_i16(&mut self) -> Result<i16, ()> {
    self.get_reader_mut().read(2)
      .map(|x| i16::from_ne_bytes(x.try_into().unwrap()))
  }
  
  fn read_i32(&mut self) -> Result<i32, ()> {
    self.get_reader_mut().read(4)
      .map(|x| i32::from_ne_bytes(x.try_into().unwrap()))
  }
  
  fn read_i64(&mut self) -> Result<i64, ()> {
    self.get_reader_mut().read(8)
      .map(|x| i64::from_ne_bytes(x.try_into().unwrap()))
  }
  
  fn read_isize(&mut self) -> Result<isize, ()> {
    self.get_reader_mut().read(size_of::<isize>())
      .map(|x| isize::from_ne_bytes(x.try_into().unwrap()))
  }
  
  fn read_f32(&mut self) -> Result<f32, ()> {
    self.get_reader_mut().read(size_of::<f32>())
      .map(|x| f32::from_ne_bytes(x.try_into().unwrap()))
  }
  
  fn read_f64(&mut self) -> Result<f64, ()> {
    self.get_reader_mut().read(size_of::<f64>())
      .map(|x| f64::from_ne_bytes(x.try_into().unwrap()))
  }
  
  fn read_bool(&mut self) -> Result<bool, ()> {
    let raw = self.read_u8()?;
    if raw == 0 {
      Ok(false)
    } else if raw == 1 {
      Ok(false)
    } else {
      Err(())
    }
  }
  
  fn read_cstr(&mut self) -> Result<&'reader std::ffi::CStr, ()> {
    // Find the length of CString
    let mut length = 0;
    loop {
      let current = self.get_reader_mut().peek(1, /* length is also offset */ length)?;
      if current[0] == 0 {
        break;
      }
      length += 1;
    }
    
    Ok(CStr::from_bytes_with_nul(self.get_reader_mut().read(length)?).unwrap())
  }
  
  fn read_str(&mut self) -> Result<&'reader str, ()> {
    let length = self.read_usize()?;
    let bytes = self.get_reader_mut().read(length)?;
    assert!(bytes.len() == length);
    str::from_utf8(bytes)
      .map_err(|_| ())
  }
  
  fn read_u8_slice(&mut self) -> Result<&'reader [u8], ()> {
    let length = self.read_usize()?;
    let bytes = self.get_reader_mut().read(length)?;
    assert!(bytes.len() == length);
    Ok(bytes)
  }
  impl_slice!(read_u16_slice, u16);
  impl_slice!(read_u32_slice, u32);
  impl_slice!(read_u64_slice, u64);
  impl_slice!(read_usize_slice, usize);
  
  fn read_i8_slice(&mut self) -> Result<&'reader [i8], ()> {
    let length = self.read_usize()?;
    Ok(bytemuck::cast_slice(self.get_reader_mut().read(length)?))
  }
  impl_slice!(read_i16_slice, i16);
  impl_slice!(read_i32_slice, i32);
  impl_slice!(read_i64_slice, i64);
  impl_slice!(read_isize_slice, isize);
  
  impl_slice!(read_f32_slice, f32);
  impl_slice!(read_f64_slice, f64);
  
  fn read_str_slice(&mut self, result: &mut Vec<&'reader str>) -> Result<(), ()> {
    let length = self.read_usize()?;
    result.reserve(length);
    for _ in 0..length {
      result.push(self.read_str()?);
    }
    
    Ok(())
  }
  
  fn read_cstr_slice(&mut self, result: &mut Vec<&'reader CStr>) -> Result<(), ()> {
    let length = self.read_usize()?;
    result.reserve(length);
    for _ in 0..length {
      result.push(self.read_cstr()?);
    }
    
    Ok(())
  }
  
  fn read_bool_slice(&mut self) -> Result<&'reader [bool], ()> {
    let bytes = self.read_u8_slice()?;
    
    // There bytes which has invalid bit pattern for bool
    if bytes.iter().find(|&&x| x != 0x00 && x != 0x01).is_some() {
      return Err(());
    }
    
    // SAFETY: Checked that it is valid bits
    Ok(unsafe { slice::from_raw_parts(bytes.as_ptr().cast::<bool>(), bytes.len()) })
  }
}


