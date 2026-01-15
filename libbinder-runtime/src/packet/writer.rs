use std::ffi::CStr;

use delegate::delegate;
use libbinder::formats::WriteFormat;

use crate::{ArcRuntime, object::Object, reference::Reference};

pub struct Writer<'packet, 'runtime: 'packet, Format: WriteFormat<'packet>, Mgr: Object<Mgr>> {
  pub(super) runtime: &'runtime ArcRuntime<Mgr>,
  pub(super) writer: libbinder::packet::writer::Writer<'packet, 'runtime, Format>
}

impl<'packet, 'runtime: 'packet, Format: WriteFormat<'packet>, Mgr: Object<Mgr>> Writer<'packet, 'runtime, Format, Mgr> {
  pub fn get_runtime(&self) -> &'runtime ArcRuntime<Mgr> {
    self.runtime
  }
  
  pub fn write_ref<T: Object<Mgr>>(&mut self, reference: &'packet Reference<Mgr, T>) -> &mut Self {
    assert!(self.runtime.ptr_eq(Reference::get_runtime(reference)), "attempt to write reference belonging to different runtime");
    self.writer.write_obj_ref(Reference::get_obj_ref(reference));
    self
  }
  
  delegate!(
    #[expr($; self)]
    to self.writer {
      pub fn write_u8(&mut self, value: u8) -> &mut Self;
      pub fn write_u16(&mut self, value: u16) -> &mut Self;
      pub fn write_u32(&mut self, value: u32) -> &mut Self;
      pub fn write_u64(&mut self, value: u64) -> &mut Self;
      pub fn write_usize(&mut self, value: usize) -> &mut Self;
      
      pub fn write_i8(&mut self, value: i8) -> &mut Self;
      pub fn write_i16(&mut self, value: i16) -> &mut Self;
      pub fn write_i32(&mut self, value: i32) -> &mut Self;
      pub fn write_i64(&mut self, value: i64) -> &mut Self;
      pub fn write_isize(&mut self, value: isize) -> &mut Self;
      
      pub fn write_f32(&mut self, value: f32) -> &mut Self;
      pub fn write_f64(&mut self, value: f64) -> &mut Self;
      pub fn write_str(&mut self, value: &str) -> &mut Self;
      pub fn write_cstr(&mut self, value: &CStr) -> &mut Self;
      pub fn write_bool(&mut self, value: bool) -> &mut Self;
      
      pub fn write_u8_slice(&mut self, value: &[u8]) -> &mut Self;
      pub fn write_u16_slice(&mut self, value: &[u16]) -> &mut Self;
      pub fn write_u32_slice(&mut self, value: &[u32]) -> &mut Self;
      pub fn write_u64_slice(&mut self, value: &[u64]) -> &mut Self;
      pub fn write_usize_slice(&mut self, value: &[usize]) -> &mut Self;
      
      pub fn write_i8_slice(&mut self, value: &[i8]) -> &mut Self;
      pub fn write_i16_slice(&mut self, value: &[i16]) -> &mut Self;
      pub fn write_i32_slice(&mut self, value: &[i32]) -> &mut Self;
      pub fn write_i64_slice(&mut self, value: &[i64]) -> &mut Self;
      pub fn write_isize_slice(&mut self, value: &[isize]) -> &mut Self;
      
      pub fn write_f32_slice(&mut self, value: &[f32]) -> &mut Self;
      pub fn write_f64_slice(&mut self, value: &[f64]) -> &mut Self;
      pub fn write_str_slice(&mut self, value: &[&str]) -> &mut Self;
      pub fn write_cstr_slice(&mut self, value: &[&CStr]) -> &mut Self;
      pub fn write_bool_slice(&mut self, value: &[bool]) -> &mut Self;
      
      pub fn write_u8_array<const LEN: usize>(&mut self, value: &[u8; LEN]) -> &mut Self;
      pub fn write_u16_array<const LEN: usize>(&mut self, value: &[u16; LEN]) -> &mut Self;
      pub fn write_u32_array<const LEN: usize>(&mut self, value: &[u32; LEN]) -> &mut Self;
      pub fn write_u64_array<const LEN: usize>(&mut self, value: &[u64; LEN]) -> &mut Self;
      pub fn write_usize_array<const LEN: usize>(&mut self, value: &[usize; LEN]) -> &mut Self;
      
      pub fn write_i8_array<const LEN: usize>(&mut self, value: &[i8; LEN]) -> &mut Self;
      pub fn write_i16_array<const LEN: usize>(&mut self, value: &[i16; LEN]) -> &mut Self;
      pub fn write_i32_array<const LEN: usize>(&mut self, value: &[i32; LEN]) -> &mut Self;
      pub fn write_i64_array<const LEN: usize>(&mut self, value: &[i64; LEN]) -> &mut Self;
      pub fn write_isize_array<const LEN: usize>(&mut self, value: &[isize; LEN]) -> &mut Self;
      
      pub fn write_f32_array<const LEN: usize>(&mut self, value: &[f32; LEN]) -> &mut Self;
      pub fn write_f64_array<const LEN: usize>(&mut self, value: &[f64; LEN]) -> &mut Self;
      pub fn write_str_array<const LEN: usize>(&mut self, value: &[&str; LEN]) -> &mut Self;
      pub fn write_cstr_array<const LEN: usize>(&mut self, value: &[&CStr; LEN]) -> &mut Self;
      pub fn write_bool_array<const LEN: usize>(&mut self, value: &[bool; LEN]) -> &mut Self;
    }
  );
}



