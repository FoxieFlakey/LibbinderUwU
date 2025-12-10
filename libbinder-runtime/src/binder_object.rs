use std::{any::Any, mem, ptr::{self, DynMetadata}, sync::Arc};

use libbinder_raw::object::reference::ObjectRefLocal;

use crate::{ArcRuntime, packet::Packet, proxy::ProxyObject};

// Any trait is used to convert bare Arc<dyn BinderObjec> to local implementation
// usually used for when receiving local reference
pub trait BinderObject<ContextManager: BinderObject<ContextManager>>: Sync + Send + Any + 'static {
  fn on_packet<'runtime>(&self, runtime: &'runtime ArcRuntime<ContextManager>, packet: &Packet<'runtime, ContextManager>) -> Packet<'runtime, ContextManager>;
}

pub trait CreateInterfaceObject<ContextManager: BinderObject<ContextManager>>: Sized {
  fn try_from_remote(runtime: &ArcRuntime<ContextManager>, remote_ref: ProxyObject<ContextManager>) -> Result<Self, ()>;
}

// This does not increments the strong count
// SAFETY: Caller must make sure local reference points to correct object
pub(crate) unsafe fn from_local_object_ref<ContextManager: BinderObject<ContextManager>>(object_ref: &ObjectRefLocal) -> Arc<dyn BinderObject<ContextManager>> {
  let raw = ptr::from_raw_parts::<dyn BinderObject<ContextManager>>(
    object_ref.data as *const (),
    // SAFETY: Lets set fire
    unsafe { mem::transmute::<*const (), DynMetadata<dyn BinderObject<ContextManager>>>(object_ref.extra_data as *const ()) }
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
pub(crate) fn into_local_object_ref<ContextManager: BinderObject<ContextManager>>(obj: &Arc<dyn BinderObject<ContextManager>>) -> ObjectRefLocal {
  let (data, vtable) = Arc::as_ptr(obj).to_raw_parts();
  ObjectRefLocal {
    data: data.addr(),
    // SAFETY: Lets set fire
    extra_data: unsafe { mem::transmute::<DynMetadata<dyn BinderObject<ContextManager>>, *const ()>(vtable) }.addr()
  }
}


