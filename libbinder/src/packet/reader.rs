use core::slice;

use bytemuck::{Pod, PodCastError};

use crate::packet::Packet;

pub struct Reader<'packet, 'binder> {
  packet: &'packet Packet<'binder>,
  full_slice: &'packet [u8],
  current_slice: &'packet [u8]
}

impl<'packet, 'binder> Reader<'packet, 'binder> {
  pub fn new(packet: &'packet Packet<'binder>) -> Self {
    Self {
      current_slice: packet.transaction.get_common().data_slice,
      full_slice: packet.transaction.get_common().data_slice,
      packet
    }
  }
}

impl Clone for Reader<'_, '_> {
  fn clone(&self) -> Self {
    Self {
      packet: self.packet,
      current_slice: self.current_slice,
      full_slice: self.full_slice
    }
  }
}

impl<'packet, 'binder> Reader<'packet, 'binder> {
  // return false if overlaps with binder objects
  // they're cannot be directly read
  //
  // currently there no way to write binder objects
  fn check_for_primitive_read_safety(&self, len: usize) -> bool {
    let current_offset = self.get_cur_offset();
    #[expect(unused)]
    let offset_range_to_check = current_offset..(current_offset + len);
    true
  }
  
  pub fn get_packet(&self) -> &'packet Packet<'binder> {
    self.packet
  }
  
  fn get_cur_offset(&self) -> usize {
    if self.current_slice.is_empty() {
      // Because slice is empty, pointer arithmetic might be wrong
      return 0;
    }
    
    assert!(!self.full_slice.is_empty());
    
    let ret = self.current_slice.as_ptr().addr() - self.full_slice.as_ptr().addr();
    assert!(ret <= self.full_slice.len());
    return ret;
  }
  
  // Skip len bytes as if reading hypothetical
  // [len; u8] primitive array
  pub fn skip_bytes(&mut self, len: usize) -> Result<(), ()>{
    const SIZE: usize = size_of::<u8>();
    
    if !self.check_for_primitive_read_safety(SIZE * len) {
      return Err(());
    }
    
    self.current_slice = &self.current_slice[SIZE..];
    Ok(())
  }
}

macro_rules! impl_primitive {
  ($name:ident, $name_array:ident, $name_array_dynamic:ident, $type:ty) => {
    pub fn $name_array_dynamic<'a>(&'a mut self, len: usize) -> Result<&'a [$type], ()> {
      let size: usize = size_of::<$type>() * len;
      
      if !self.check_for_primitive_read_safety(size) {
        return Err(());
      }
      
      let sliced_ptr = self.current_slice[..size].as_ptr().cast::<$type>();
      self.current_slice = &self.current_slice[size..];
      
      // SAFETY: We've previously checked if the slice is large enough
      // via bound checking on slice
      //
      // This is those small nice thing of Rust :3
      Ok(unsafe { slice::from_raw_parts(sliced_ptr, len) })
    }
    
    pub fn $name_array<'a, const LEN: usize>(&'a mut self) -> Result<&'a [$type; LEN], ()> {
      self.$name_array_dynamic(LEN)
        .map(|x| {
          let ret: &'a [$type; LEN] = x.try_into().unwrap();
          ret
        })
    }
    
    pub fn $name(&mut self) -> Result<$type, ()> {
      let size: usize = size_of::<$type>();
      
      if !self.check_for_primitive_read_safety(size) {
        return Err(());
      }
      
      let ret = <$type>::from_ne_bytes(self.current_slice[..size].try_into().unwrap());
      self.current_slice = &self.current_slice[size..];
      Ok(ret)
    }
  };
}

impl Reader<'_, '_> {
  impl_primitive!(read_u8, read_u8_array, read_u8_slice, u8);
  impl_primitive!(read_u16, read_u16_array, read_u16_slice, u16);
  impl_primitive!(read_u32, read_u32_array, read_u32_slice, u32);
  impl_primitive!(read_u64, read_u64_array, read_u64_slice, u64);
  impl_primitive!(read_u128, read_u128_array, read_u128_slice, u128);
  impl_primitive!(read_usize, read_usize_array, read_usize_slice, usize);
  
  impl_primitive!(read_i8, read_i8_array, read_i8_slice, i8);
  impl_primitive!(read_i16, read_i16_array, read_i16_slice, i16);
  impl_primitive!(read_i32, read_i32_array, read_i32_slice, i32);
  impl_primitive!(read_i64, read_i64_array, read_i64_slice, i64);
  impl_primitive!(read_i128, read_i128_array, read_i128_slice, i128);
  impl_primitive!(read_isize, read_isize_array, read_isize_slice, isize);
  
  impl_primitive!(read_f32, read_f32_array, read_f32_slice, f32);
  impl_primitive!(read_f64, read_f64_array, read_f64_slice, f64);
  
  pub fn read_pod_copy<'a, T: Pod>(&'a mut self) -> Result<T, ()> {
    let size: usize = size_of::<T>();
    
    if !self.check_for_primitive_read_safety(size) {
      return Err(());
    }
    
    let bytes = &self.current_slice[..size];
    self.current_slice = &self.current_slice[size..];
    Ok(bytemuck::pod_read_unaligned(bytes))
  }
  
  // If Ok(Err) returned, then the current offset is not updated
  // caller may decide to call read_pod_copy instead which allows
  // unaligned read, as alignment might not be right
  pub fn read_pod<'a, T: Pod>(&'a mut self) -> Result<Result<&'a T, PodCastError>, ()> {
    let size: usize = size_of::<T>();
    
    if !self.check_for_primitive_read_safety(size) {
      return Err(());
    }
    
    let ret = bytemuck::try_from_bytes(&self.current_slice[..size]);
    if ret.is_ok() {
      self.current_slice = &self.current_slice[size..];
    }
    Ok(ret)
  }
  
  // Return Ok(Some(val)) if entire slice of bool succesfully read
  // but return Ok(None), if slice bool readable but it
  // would cause UB, the byte is not 0x00 and not 0x01
  // in that case the current read position is not updated
  pub fn read_bool_slice<'a>(&'a mut self, len: usize) -> Result<Option<&'a [bool]>, ()> {
    let size: usize = size_of::<u8>() * len;
    
    if !self.check_for_primitive_read_safety(size) {
      return Err(());
    }
    
    if !self.check_for_primitive_read_safety(size) {
      return Err(());
    }
    
    let sliced_ptr = self.current_slice[..size].as_ptr().cast::<u8>();
    self.current_slice = &self.current_slice[size..];
    
    // SAFETY: We've previously checked if the slice is large enough
    // via bound checking on slice
    //
    // This is those small nice thing of Rust :3
    let u8_slice = unsafe { slice::from_raw_parts(sliced_ptr, len) };
    let is_valid = u8_slice.iter()
      .filter(|&&x| x != 0x00 || x != 0x01)
      .next()
      .is_none();
    
    if !is_valid {
      Ok(None)
    } else {
      // SAFETY: Previously checked that data in the slice is valid bool
      Ok(Some(unsafe { slice::from_raw_parts(u8_slice.as_ptr().cast::<bool>(), len) }))
    }
  }
  
  // Return Ok(Some(val)) if entire array of bool succesfully read
  // but return Ok(None), if array bool readable but it
  // would cause UB, the byte is not 0x00 and not 0x01
  // in that case the current read position is not updated
  pub fn read_bool_array<'a, const LEN: usize>(&'a mut self) -> Result<Option<&'a [bool; LEN]>, ()> {
    self.read_bool_slice(LEN)
      .map(|x| {
        x.map(|x| {
          let ret: &[bool; LEN] = x.try_into().unwrap();
          ret
        })
      })
  }
  
  // Return Ok(Some(val)) if bool succesfully read
  // but return Ok(None), if bool readable but it
  // would cause UB, the byte is not 0x00 and not 0x01
  // in that case the current read position is not updated
  pub fn read_bool(&mut self) -> Result<Option<bool>, ()> {
    const SIZE: usize = size_of::<u8>();
    
    if !self.check_for_primitive_read_safety(SIZE) {
      return Err(());
    }
    
    let ret = <u8>::from_ne_bytes(self.current_slice[..SIZE].try_into().unwrap());
    if ret == 0x00 {
      self.current_slice = &self.current_slice[SIZE..];
      Ok(Some(false))
    } else if ret == 0x01 {
      self.current_slice = &self.current_slice[SIZE..];
      Ok(Some(true))
    } else {
      // Byte has invalid pattern
      Ok(None)
    }
  }
}

