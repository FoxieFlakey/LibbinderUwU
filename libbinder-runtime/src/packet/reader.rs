use std::ffi::CStr;

use delegate::delegate;
use libbinder::formats::{ReadFormat, SliceReadResult};

use crate::{ArcRuntime, object::Object};

pub struct Reader<'packet, 'runtime: 'packet, Format: ReadFormat<'packet>, Mgr: Object<Mgr>> {
  pub(super) runtime: &'runtime ArcRuntime<Mgr>,
  pub(super) reader: libbinder::packet::reader::Reader<'packet, 'runtime, Format>
}

impl<'packet, 'runtime: 'packet, Format: ReadFormat<'packet>, Mgr: Object<Mgr>> Reader<'packet, 'runtime, Format, Mgr> {
  pub fn get_runtime(&self) -> &'runtime ArcRuntime<Mgr> {
    self.runtime
  }
  
  delegate!(
    to self.reader {
      pub fn read_u8(&mut self) -> Result<u8, ()>;
      pub fn read_u16(&mut self) -> Result<u16, ()>;
      pub fn read_u32(&mut self) -> Result<u32, ()>;
      pub fn read_u64(&mut self) -> Result<u64, ()>;
      pub fn read_usize(&mut self) -> Result<usize, ()>;
      
      pub fn read_i8(&mut self) -> Result<i8, ()>;
      pub fn read_i16(&mut self) -> Result<i16, ()>;
      pub fn read_i32(&mut self) -> Result<i32, ()>;
      pub fn read_i64(&mut self) -> Result<i64, ()>;
      pub fn read_isize(&mut self) -> Result<isize, ()>;
      
      pub fn read_f32(&mut self) -> Result<f32, ()>;
      pub fn read_f64(&mut self) -> Result<f64, ()>;
      pub fn read_str(&mut self) -> Result<&'packet str, ()>;
      pub fn read_cstr(&mut self) -> Result<&'packet CStr, ()>;
      pub fn read_bool(&mut self) -> Result<bool, ()>;
      
      pub fn read_u8_slice(&mut self) -> Result<&'packet [u8], ()>;
      pub fn read_u16_slice(&mut self) -> Result<SliceReadResult<'packet, u16>, ()>;
      pub fn read_u32_slice(&mut self) -> Result<SliceReadResult<'packet, u32>, ()>;
      pub fn read_u64_slice(&mut self) -> Result<SliceReadResult<'packet, u64>, ()>;
      pub fn read_usize_slice(&mut self) -> Result<SliceReadResult<'packet, usize>, ()>;
      
      pub fn read_i8_slice(&mut self) -> Result<&'packet [i8], ()>;
      pub fn read_i16_slice(&mut self) -> Result<SliceReadResult<'packet, i16>, ()>;
      pub fn read_i32_slice(&mut self) -> Result<SliceReadResult<'packet, i32>, ()>;
      pub fn read_i64_slice(&mut self) -> Result<SliceReadResult<'packet, i64>, ()>;
      pub fn read_isize_slice(&mut self) -> Result<SliceReadResult<'packet, isize>, ()>;
      
      pub fn read_f32_slice(&mut self) -> Result<SliceReadResult<'packet, f32>, ()>;
      pub fn read_f64_slice(&mut self) -> Result<SliceReadResult<'packet, f64>, ()>;
      pub fn read_str_slice(&mut self) -> Result<Vec<&'packet str>, ()>;
      pub fn read_cstr_slice(&mut self) -> Result<Vec<&'packet CStr>, ()>;
      pub fn read_bool_slice(&mut self) -> Result<&'packet [bool], ()>;
    }
  );
}



