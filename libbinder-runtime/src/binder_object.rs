use std::{mem, ptr::{self, DynMetadata}, sync::Arc};

use libbinder::packet::{Packet, builder::PacketBuilder};
use libbinder_raw::ObjectRefLocal;

use crate::Runtime;

pub trait BinderObject: Sync + Send + 'static {
  fn on_packet(&self, runtime: &Runtime, packet: &Packet<'_>, reply_builder: &mut PacketBuilder);
}

// This does increments the strong count
// SAFETY: Caller must make sure local reference points to correct object
pub(crate) unsafe fn from_local_object_ref(object_ref: &ObjectRefLocal) -> Arc<dyn BinderObject> {
  let raw = ptr::from_raw_parts::<dyn BinderObject>(
    object_ref.data as *const (),
    // SAFETY: Lets set fire
    unsafe { mem::transmute::<*const (), DynMetadata<dyn BinderObject>>(object_ref.extra_data as *const ()) }
  );
  
  // SAFETY: Caller already make sure it points to correct object
  unsafe { Arc::from_raw(raw) }
}

// It creates local reference, but does not keep the object
// alive. Caller should keep 'obj' alive as long as the reference
//
// This does not increments the counter. Caller should mem::forget
// if they manage the reference
//
// Also the object musn't move at all
pub(crate) fn into_local_object_ref(obj: &Arc<dyn BinderObject>) -> ObjectRefLocal {
  let (data, vtable) = Arc::as_ptr(obj).to_raw_parts();
  ObjectRefLocal {
    data: data.addr(),
    // SAFETY: Lets set fire
    extra_data: unsafe { mem::transmute::<DynMetadata<dyn BinderObject>, *const ()>(vtable) }.addr()
  }
}


