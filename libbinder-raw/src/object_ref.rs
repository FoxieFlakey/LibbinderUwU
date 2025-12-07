use bytemuck::{Pod, Zeroable};
use enumflags2::{BitFlag, BitFlags, bitflags};

use crate::{BinderUsize, ObjectHeader, object};

#[bitflags]
#[repr(u32)]
#[derive(Clone, Copy)]
pub enum ObjectRefFlags {
  AcceptFds = 0x100,
  SendSecurityContext = 0x1000,
  
  // Priorities, bits (note, these together forms an u8 describing preferred scheduling
  // priority, 0 is highest and 0xFF is lowest)
  //
  // Little side note: this is a suggestion :3 kernel may-or-may-not boost depends other
  // stuffs
  PriorityBit0 = 0x01,
  PriorityBit1 = 0x02,
  PriorityBit2 = 0x04,
  PriorityBit3 = 0x08,
  PriorityBit4 = 0x10,
  PriorityBit5 = 0x20,
  PriorityBit6 = 0x40,
  PriorityBit7 = 0x80
}

impl ObjectRefFlags {
  pub fn get_priority(obj: BitFlags<Self>) -> u8 {
    (obj.bits() & 0xFF) as u8
  }
  
  pub fn from_priority_bits(priority: u8) -> BitFlags<Self> {
    ObjectRefFlags::from_bits(priority as u32).ok().expect("cannot occur")
  }
}

#[derive(Clone)]
pub enum ObjectRef {
  Local(ObjectRefLocal),
  Remote(ObjectRefRemote)
}

impl ObjectRef {
  pub fn with_raw_bytes<R, F: FnOnce(&[u8]) -> R>(&self, func: F) -> R {
    let raw = match self {
      ObjectRef::Local(x) => x.into_raw(),
      ObjectRef::Remote(x) => x.into_raw()
    };
    
    let ret = func(bytemuck::bytes_of(&raw));
    ret
  }
}

#[derive(Clone)]
pub struct ObjectRefLocal {
  // Whatever data can be in these, kernel won't touch it
  pub data: usize,
  pub extra_data: usize
}

#[derive(Clone)]
pub struct ObjectRefRemote {
  pub data_handle: u32
}

pub const CONTEXT_MANAGER_REF: ObjectRefRemote = ObjectRefRemote { data_handle: 0 };

impl ObjectRefLocal {
  pub(crate) fn into_raw(&self) -> ObjectRefRaw {
    ObjectRefRaw {
      header: ObjectHeader {
        kind: object::BINDER
      },
      flags: 0,
      binder_or_handle: BinderOrHandleUnion {
        binder: self.data
      },
      extra_data: self.extra_data
    }
  }
}

impl ObjectRefRemote {
  pub(crate) fn into_raw(&self) -> ObjectRefRaw {
    ObjectRefRaw {
      header: ObjectHeader {
        kind: object::HANDLE
      },
      flags: 0,
      binder_or_handle: BinderOrHandleUnion {
        handle: self.data_handle
      },
      extra_data: 0
    }
  }
}

// Equivalent to struct flat_binder_object
// it is to Foxie's understanding kind of reference
// to an object either remote/local and kernel does
// not copy the private data referenced by this
//
// So can be thought of object reference
#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct ObjectRefRaw {
  header: ObjectHeader,
  flags: u32,
  binder_or_handle: BinderOrHandleUnion,
  
  // On local process (the owner of private object), can possibly have
  // arbitrary data here. With Box<dyn Trait>, a vtable part of trait can
  // be stored here or incase of array liek Box<[Item]> length can be
  // stored here and it won't be sent to other process
  //
  // ACCORDING to ChatGPT, chatgpt been really helpful to translate the
  // meaning to be more friendly for me because I couldn't understand
  // strange new concepts like  binder or binder handle, flat binder
  // object?? i thought flat binder object contains flattened data when
  //
  // in reality...
  // 1. "binder" means a reference to private object owned by the receiver
  //    thus kernel can give pointer back
  // 2. "binder handle" means a remote reference to private object and instead
  //    raw pointer, non owner process is given handle
  // 3. "flat binder object" means essentially a boxed reference much like Java's
  //    StrongReference (hypothetical one subclassed from Reference<T>) instead "directly"
  //    storing the data, it is storing object containing reference to it.
  //    In Rust it would be like Box<&T> not T
  //
  // Kernel does not care what is put in extra_data and pointer to object. Heck it
  // does not have to valid pointer. Kernel won't touch it ^w^
  extra_data: BinderUsize
}

// It is a union inside flat_binder_object
#[repr(C)]
#[derive(Copy, Clone, Zeroable)]
union BinderOrHandleUnion {
  binder: BinderUsize,
  handle: u32
}

unsafe impl Pod for BinderOrHandleUnion {}

