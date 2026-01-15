use std::{mem::{ManuallyDrop, forget}, ptr, sync::{Arc, atomic::{AtomicU8, Ordering}}};

use libbinder_raw::types::reference::ObjectRefLocal;

use crate::object::Object;

struct ObjectData<Mgr: Object<Mgr>> {
  reference: Arc<dyn Object<Mgr>>,
  ref_tracker: AtomicU8
}

const REF_STRONG_BIT: u8 = 0x01;
const REF_WEAK_BIT: u8 = 0x02;
const REF_DEAD_BIT: u8 = 0x04;
const REF_UNDER_CONSTRUCTION: u8 = 0x08;

pub(crate) struct BoxedObject<Mgr: Object<Mgr>> {
  inner: Box<ObjectData<Mgr>>
}

impl<Mgr: Object<Mgr>> BoxedObject<Mgr> {
  pub fn new<T: Object<Mgr>>(obj: Arc<T>) -> Self {
    BoxedObject {
      inner: Box::new(ObjectData {
        reference: obj,
        ref_tracker: AtomicU8::new(REF_UNDER_CONSTRUCTION)
      })
    }
  }
  
  pub fn done_constructing(&self) {
    let ret = self.inner.ref_tracker.fetch_and(!REF_UNDER_CONSTRUCTION, Ordering::Relaxed);
    assert!(ret & REF_UNDER_CONSTRUCTION != 0);
  }
  
  fn check_for_access(&self) {
    Self::check_bits_for_access(self.inner.ref_tracker.load(Ordering::Relaxed));
  }
  
  fn check_bits_for_access(bits: u8) {
    if bits & REF_UNDER_CONSTRUCTION != 0 {
      panic!("Object is under construction");
    }
    
    if bits & REF_DEAD_BIT != 0 {
      panic!("Attempt to use dead boxed object!");
    }
  }
  
  pub fn get_object(&self) -> &Arc<dyn Object<Mgr>> {
    self.check_for_access();
    &self.inner.reference
  }
  
  pub unsafe fn into_raw(self) -> ObjectRefLocal {
    self.check_for_access();
    let ret = Box::as_ptr(&self.inner);
    forget(self);
    
    ObjectRefLocal {
      data: ret.expose_provenance(),
      extra_data: 0
    }
  }
  
  // These four methods maps to BC_INCREFS, BC_DECREFS, BC_ACQUIRE, BC_RELEASE
  pub fn on_bc_increfs(&self) {
    self.inner.ref_tracker.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| {
      Self::check_bits_for_access(x);
      if x & REF_WEAK_BIT != 0 {
        panic!("Kernel sent BC_INCREFS, when there already weak reference");
      }
      Some(x | REF_WEAK_BIT)
    }).unwrap();
  }
  
  pub fn on_bc_decrefs(&self) {
    self.inner.ref_tracker.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| {
      Self::check_bits_for_access(x);
      if x & REF_WEAK_BIT == 0 {
        panic!("Kernel sent BC_DECREFS, when there already no weak reference");
      }
      Some(x & !REF_WEAK_BIT)
    }).unwrap();
  }
  
  pub fn on_bc_acquire(&self) {
    self.inner.ref_tracker.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| {
      Self::check_bits_for_access(x);
      if x & REF_STRONG_BIT != 0 {
        panic!("Kernel sent BC_ACQUIRE, when there already strong reference");
      }
      Some(x | REF_STRONG_BIT)
    }).unwrap();
  }
  
  pub fn on_bc_release(&self) {
    self.inner.ref_tracker.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| {
      Self::check_bits_for_access(x);
      if x & REF_STRONG_BIT == 0 {
        panic!("Kernel sent BC_RELEASE, when there already no strong reference");
      }
      Some(x & !REF_STRONG_BIT)
    }).unwrap();
  }
  
  pub fn is_dead(&self) -> bool {
    self.inner.ref_tracker.load(Ordering::Relaxed) & REF_DEAD_BIT != 0
  }
  
  // Dropping Self would decrease the Arc counter on the
  // object, to avoid mistake it is wrapped inside ManuallyDrop
  #[must_use = "this has very intricate behaviour, please check"]
  pub unsafe fn from_raw(local_ref: ObjectRefLocal) -> ManuallyDrop<Self> {
    let ret = ManuallyDrop::new(Self {
      inner: unsafe { Box::from_raw(ptr::with_exposed_provenance_mut(local_ref.data)) }
    });
    ret.check_for_access();
    ret
  }
}
