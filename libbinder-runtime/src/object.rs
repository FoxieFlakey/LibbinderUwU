use std::{any::Any, error::Error, mem::{ManuallyDrop, forget}, ptr, sync::Arc};
use libbinder_raw::types::reference::ObjectRefLocal;

use crate::packet::Packet;

#[derive(Debug)]
pub enum TransactionError {
  // The target of reply/transaction, no longer exist
  DeadTarget,
  
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

pub(crate) struct BoxedObject<Mgr: Object<Mgr>> {
  inner: Box<Arc<dyn Object<Mgr>>>
}

impl<Mgr: Object<Mgr>> BoxedObject<Mgr> {
  pub fn new<T: Object<Mgr>>(obj: Arc<T>) -> Self {
    BoxedObject {
      inner: Box::new(obj)
    }
  }
  
  pub fn get_object(&self) -> &Arc<dyn Object<Mgr>> {
    &self.inner
  }
  
  pub unsafe fn into_raw(self) -> ObjectRefLocal {
    let ret = Box::as_ptr(&self.inner);
    forget(self);
    
    ObjectRefLocal {
      data: ret.expose_provenance(),
      extra_data: 0
    }
  }
  
  // Dropping Self would decrease the Arc counter on the
  // object, to avoid mistake it is wrapped inside ManuallyDrop
  pub unsafe fn from_raw(local_ref: ObjectRefLocal) -> ManuallyDrop<Self> {
    ManuallyDrop::new(Self {
      inner: unsafe { Box::from_raw(ptr::with_exposed_provenance_mut(local_ref.data)) }
    })
  }
}


