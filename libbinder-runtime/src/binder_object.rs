use std::{mem, ptr::{self, DynMetadata}};

use libbinder::packet::{Packet, builder::PacketBuilder};
use libbinder_raw::ObjectRefLocal;

use crate::Runtime;

pub trait BinderObject: Sync + Send + 'static {
  fn on_packet(&self, runtime: &Runtime, packet: &Packet<'_>, reply_builder: &mut PacketBuilder<'_>);
}

pub(crate) fn from_local_object_ref(object_ref: &ObjectRefLocal) -> *const dyn BinderObject {
  ptr::from_raw_parts::<dyn BinderObject>(
    object_ref.data as *const (),
    // SAFETY: Lets set fire
    unsafe { mem::transmute::<*const (), DynMetadata<dyn BinderObject>>(object_ref.extra_data as *const ()) }
  )
}

// It creates local reference, but does not keep the object
// alive. Caller should keep 'obj' alive as long as the reference
//
// Also the object musn't move at all
pub(crate) fn into_local_object_ref(obj: *const dyn BinderObject) -> ObjectRefLocal {
  let (data, vtable) = obj.to_raw_parts();
  ObjectRefLocal {
    data: data.addr(),
    // SAFETY: Lets set fire
    extra_data: unsafe { mem::transmute::<DynMetadata<dyn BinderObject>, *const ()>(vtable) }.addr()
  }
}


