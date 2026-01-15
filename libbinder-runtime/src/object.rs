use std::{any::Any, fmt::{Debug, Display}, mem, ptr::{self, DynMetadata}, sync::Arc};

use libbinder_raw::types::reference::ObjectRefLocal;

use crate::{packet::Packet, proxy::Proxy};

pub enum TransactionError {
  // The target of reply/transaction, no longer exist
  UnreachableTarget,
  
  // The transaction is sent, but then target dies
  // no reply is given but the transaction did sent
  NoReply,
  
  // Transaction did not sent at all
  FailedReply,
  
  // The reply was malformed
  MalformedReply,
  
  // Error message from local, in this case the transaction did not get sent
  // runtime never uses this, it exists for convenience
  LocalError(Box<dyn Display>),
  
  // Error message from remote target, in this case the transaction did get sent
  // but remote errored out
  // runtime never uses this, it exists for convenience
  RemoteError(Box<dyn Display>)
}

impl Debug for TransactionError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      TransactionError::UnreachableTarget => writeln!(f, "UnreachableTarget"),
      TransactionError::NoReply => writeln!(f, "NoReply"),
      TransactionError::FailedReply =>  writeln!(f, "FailedReply"),
      TransactionError::MalformedReply =>  writeln!(f, "MalformedReply"),
      TransactionError::LocalError(display) => display.fmt(f),
      TransactionError::RemoteError(display) => display.fmt(f)
    }
  }
}

// About storing ArcRuntime, caller should store only weak
// reference to the runtime, don't store strong reference
//
// Runtime will store the strong reference to object if its
// sent outside
pub trait Object<Mgr: Object<Mgr>>: Sync + Send + Any + 'static {
  fn do_transaction<'packet, 'runtime>(&self, packet: &'packet Packet<'runtime, Mgr>) -> Result<Option<Packet<'runtime, Mgr>>, TransactionError>;
}

pub trait FromProxy<Mgr: Object<Mgr>>: Object<Mgr> + Sized {
  fn from_proxy(proxy: Proxy<Mgr>) -> Result<Self, ()>;
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


