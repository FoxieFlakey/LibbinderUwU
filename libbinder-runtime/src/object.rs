use std::{any::Any, error::Error, mem, ptr::{self, DynMetadata}, sync::Arc};

use libbinder_raw::types::reference::ObjectRefLocal;

use crate::packet::Packet;

#[derive(Debug)]
pub enum TransactionError {
  // The target of reply/transaction, no longer exist
  UnreachableTarget,
  
  // The transaction is sent, but then target dies
  // no reply is given but the transaction did sent
  NoReply,
  
  // Transaction did not sent at all
  FailedReply,
  
  // Miscellanous error
  MiscellanousError(Box<dyn Error>)
}

// About storing ArcRuntime, caller should store only weak
// reference to the runtime, don't store strong reference
//
// Runtime will store the strong reference to object if its
// sent outside
pub trait Object<Mgr: Object<Mgr>>: Sync + Send + Any + 'static {
  fn do_transaction<'packet, 'runtime>(&self, packet: &'packet Packet<'runtime, Mgr>) -> Result<Packet<'runtime, Mgr>, TransactionError>;
}

// Does not touch the reference counter
pub(crate) unsafe fn from_local_ref<Mgr: Object<Mgr>>(local_ref: ObjectRefLocal) -> Arc<dyn Object<Mgr>> {
  let raw = ptr::from_raw_parts::<dyn Object<Mgr>>(
    local_ref.data as *const (),
    // SAFETY: Lets set fire
    unsafe { mem::transmute::<*const (), DynMetadata<dyn Object<Mgr>>>(local_ref.extra_data as *const ()) }
  );
  
  unsafe { Arc::from_raw(raw) }
}

// It leaks the Arc
pub(crate) fn into_local_ref<Mgr: Object<Mgr>>(obj: Arc<dyn Object<Mgr>>) -> ObjectRefLocal {
  let (data, vtable) = Arc::into_raw(obj).to_raw_parts();
  ObjectRefLocal {
    data: data.addr(),
    
    // SAFETY: Lets set fire
    extra_data: unsafe { mem::transmute::<DynMetadata<dyn Object<Mgr>>, *const ()>(vtable) }.addr()
  }
}


