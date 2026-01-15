use std::{mem::{ManuallyDrop, forget}, ptr, sync::Arc};

use libbinder_raw::types::reference::ObjectRefLocal;

use crate::object::Object;

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
