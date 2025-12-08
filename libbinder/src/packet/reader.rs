use std::ffi::CStr;

use libbinder_raw::types::{Type, reference::ObjectRef as ObjectRefRaw};

use crate::{formats::{InnerReader, ReadFormat, SliceReadResult}, packet::Packet};

#[derive(Clone)]
pub struct Reader<'packet, 'binder, Format: ReadFormat<'packet>> {
  // There two instance of format
  // the 'format' is the current one
  // while 'saved_format' is saved one before
  // read begins. If there any error reading its
  // rolled back to 'saved_format'
  
  format: Format,
  saved_format: Format,
  packet: &'packet Packet<'binder>
}

#[derive(Clone)]
struct ReaderState<'packet> {
  packet: &'packet Packet<'packet>,
  full_slice: &'packet [u8],
  current_slice: &'packet [u8]
}

impl ReaderState<'_> {
  // return false if overlaps with binder objects
  // they're cannot be directly read
  fn check_for_primitive_read_safety(&self, len: usize, is_peeking: Option<usize>) -> bool {
    let current_offset = self.get_cur_offset(is_peeking);
    let offset_range_to_check = current_offset..(current_offset + len);
    
    for &offset in self.packet.offset_buffer.iter() {
      if offset > offset_range_to_check.start {
        // there no need to go further
        break;
      }
      
      let Some(header) = self.packet.data_buffer.get(offset..Type::bytes_needed())
        .map(|x: &[u8]| x.try_into().ok())
        .flatten()
      else {
        // Offset doesn't make sense lets be conservative and assume its not safe
        return false;
      };
      
      let object_type = Type::from_bytes(header);
      let size_of_object = object_type.type_size_with_header();
      let range_occupied = offset..offset+size_of_object;
      
      if range_occupied.contains(&offset_range_to_check.start) || range_occupied.contains(&(offset_range_to_check.end - 1)) {
        // Overlaps with binder objects which is 'not safe' to read
        return false;
      }
    }
    
    true
  }
  
  fn get_cur_offset(&self, is_peeking: Option<usize>) -> usize {
    let slice = &self.current_slice[is_peeking.unwrap_or(0)..];
    if slice.is_empty() {
      // Because slice is empty, pointer arithmetic might be wrong
      return 0;
    }
    
    assert!(!self.full_slice.is_empty());
    
    let ret = slice.as_ptr().addr() - self.full_slice.as_ptr().addr();
    assert!(ret <= self.full_slice.len());
    return ret;
  }
}

impl<'packet> InnerReader<'packet> for ReaderState<'packet> {
  fn clone_reader(&self) -> Box<dyn InnerReader<'packet>> {
    Box::new(self.clone())
  }
  
  fn read(&mut self, size: usize) -> Result<&'packet [u8], ()> {
    if !self.check_for_primitive_read_safety(size, None) {
      return Err(());
    }
    
    let ret = &self.current_slice[..size];
    self.current_slice = &self.current_slice[size..];
    Ok(ret)
  }
  
  fn peek(&self, size: usize, offset: usize) -> Result<&'packet [u8], ()> {
    if !self.check_for_primitive_read_safety(size, Some(offset)) {
      return Err(());
    }
    
    Ok(&self.current_slice[offset..(offset + size)])
  }
}

impl<'packet, 'binder, Format: ReadFormat<'packet>> Reader<'packet, 'binder, Format> {
  pub fn new(packet: &'packet Packet<'binder>, mut format: Format) -> Self {
    let data_slice = packet.transaction.get_common().data_slice;
    format.set_reader(Box::new(ReaderState {
      current_slice: data_slice,
      full_slice: data_slice,
      packet
    }));
    
    Self {
      packet,
      saved_format: format.clone(),
      format
    }
  }
  
  pub fn get_packet(&self) -> &'packet Packet<'packet> {
    self.packet
  }
}

macro_rules! forward {
  ($name:ident, $type:ty) => {
    pub fn $name(&mut self) -> Result<$type, ()> {
      self.format.$name()
        .inspect_err(|_| self.format = self.saved_format.clone())
        .inspect(|_| self.saved_format = self.format.clone())
    }
  };
}

impl<'packet, 'binder, Format: ReadFormat<'packet>> Reader<'packet, 'binder, Format> {
  forward!(read_u8, u8);
  forward!(read_u16, u16);
  forward!(read_u32, u32);
  forward!(read_u64, u64);
  forward!(read_usize, usize);
  
  forward!(read_i8, i8);
  forward!(read_i16, i16);
  forward!(read_i32, i32);
  forward!(read_i64, i64);
  forward!(read_isize, isize);
  
  forward!(read_f32, f32);
  forward!(read_f64, f64);
  forward!(read_str, &'packet str);
  forward!(read_cstr, &'packet CStr);
  forward!(read_bool, bool);
  
  forward!(read_u8_slice, &'packet [u8]);
  forward!(read_u16_slice, SliceReadResult<'packet, u16>);
  forward!(read_u32_slice, SliceReadResult<'packet, u32>);
  forward!(read_u64_slice, SliceReadResult<'packet, u64>);
  forward!(read_usize_slice, SliceReadResult<'packet, usize>);
  
  forward!(read_i8_slice, &'packet [i8]);
  forward!(read_i16_slice, SliceReadResult<'packet, i16>);
  forward!(read_i32_slice, SliceReadResult<'packet, i32>);
  forward!(read_i64_slice, SliceReadResult<'packet, i64>);
  forward!(read_isize_slice, SliceReadResult<'packet, isize>);
  
  forward!(read_f32_slice, SliceReadResult<'packet, f32>);
  forward!(read_f64_slice, SliceReadResult<'packet, f64>);
  
  pub fn read_cstr_slice(&mut self) -> Result<Vec<&'packet CStr>, ()> {
    let mut res = Vec::new();
    self.format.read_cstr_slice(&mut res)
      .inspect_err(|_| self.format = self.saved_format.clone())
      .inspect(|_| self.saved_format = self.format.clone())?;
    Ok(res)
  }
  
  pub fn read_str_slice(&mut self) -> Result<Vec<&'packet str>, ()> {
    let mut res = Vec::new();
    self.format.read_str_slice(&mut res)
      .inspect_err(|_| self.format = self.saved_format.clone())
      .inspect(|_| self.saved_format = self.format.clone())?;
    Ok(res)
  }
  
  forward!(read_bool_slice, &'packet [bool]);
  
  pub fn read_reference(&mut self) -> Result<crate::ObjectRef<'binder>, ()> {
    let ref_obj = Type::try_from_bytes(self.format.get_reader().peek(Type::bytes_needed(), 0)?)?;
    match ref_obj {
      Type::LocalReference | Type::RemoteReference => {
        let type_size = ref_obj.type_size_with_header();
        let bytes = self.format.get_reader().peek(type_size, 0)?;
        let result = ObjectRefRaw::try_from_bytes(bytes)?;
        
        // The data was successfully read, lets just advance the reader state
        self.format.get_reader().read(type_size).unwrap();
        Ok(crate::ObjectRef::new(self.packet.binder_dev, result))
      }
      _ => Err(())
    }
  }
}

