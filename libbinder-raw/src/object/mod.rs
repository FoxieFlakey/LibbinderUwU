use bytemuck::{Pod, Zeroable};
use bytemuck_utils::PodData;

const TYPE_LARGE: u8 = 0x85;

const fn pack_chars(c1: u8, c2: u8, c3: u8, c4: u8) -> u32 {
  ((c1 as u32) << 24) |
  ((c2 as u32) << 16) |
  ((c3 as u32) << 8) |
  (c4 as u32)
}

pub(crate)  const BINDER: u32 = pack_chars(b's', b'b', b'*', TYPE_LARGE);
pub(crate)  const WEAK_BINDER: u32 = pack_chars(b'w', b'b', b'*', TYPE_LARGE);
pub(crate)  const HANDLE: u32 = pack_chars(b's', b'h', b'*', TYPE_LARGE);
pub(crate)  const WEAK_HANDLE: u32 = pack_chars(b'w', b'h', b'*', TYPE_LARGE);
pub(crate)  const FD: u32 = pack_chars(b'f', b'd', b'*', TYPE_LARGE);
pub(crate)  const FDA: u32 = pack_chars(b'f', b'd', b'a', TYPE_LARGE);
pub(crate)  const PTR: u32 = pack_chars(b'p', b't', b'*', TYPE_LARGE);

pub mod reference;

pub enum Type {
  RemoteReference,
  LocalReference,
  WeakRemoteReference,
  WeakLocalReference,
  FileDescriptor,
  FileDescriptorArray,
  ByteBuffer
}

impl Type {
  pub fn bytes_needed() -> usize {
    size_of::<ObjectHeaderRaw>()
  }
  
  pub fn try_from_bytes(bytes: &[u8]) -> Result<Type, ()> {
    let raw = PodData::<ObjectHeaderRaw>::from_bytes(bytes);
    match raw.kind {
      BINDER => Ok(Type::LocalReference),
      HANDLE => Ok(Type::RemoteReference),
      WEAK_BINDER => Ok(Type::WeakLocalReference),
      WEAK_HANDLE => Ok(Type::WeakRemoteReference),
      FD => Ok(Type::FileDescriptor),
      FDA => Ok(Type::FileDescriptorArray),
      PTR => Ok(Type::ByteBuffer),
      _ => Err(())
    }
  }
}

// Equivalent to struct binder_object_header
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(crate) struct ObjectHeaderRaw {
  pub(crate) kind: u32
}

