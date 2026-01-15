#![feature(box_as_ptr)]

use std::{collections::HashMap, mem::ManuallyDrop, os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd}, ptr, sync::{Arc, Mutex, RwLock, Weak}, thread::{self, JoinHandle}};

use libbinder::packet::builder::PacketBuilder as libbinder_PacketBuilder;
use libbinder_raw::types::reference::{CONTEXT_MANAGER_REF, ObjectRefLocal};
use nix::libc;
use thread_local::ThreadLocal;

use crate::{object::Object, boxed_object::BoxedObject, packet::builder::PacketBuilder, proxy::{Proxy, SelfMananger}, util::OwnedMmap, worker::worker};

pub mod object;
pub mod packet;
pub mod proxy;

mod util;
mod worker;
mod boxed_object;
mod context;

pub(crate) struct Shared<Mgr: Object<Mgr>> {
  pub(crate) binder_dev: Arc<OwnedFd>,
  mgr: Arc<Mgr>,
  
  mgr_local_ref: Option<ObjectRefLocal>,
  
  // Used by Binder to store incoming transaction and buffer :3
  // don't need to be used
  _binder_mem: OwnedMmap,
  shutdown_pipe_wr: OwnedFd,
  _shutdown_pipe_ro: Arc<OwnedFd>,
  worker: Mutex<Option<JoinHandle<()>>>,
  
  local_objects_sent_outside: RwLock<HashMap<ObjectRefLocal, BoxedObject<Mgr>>>,
  
  exec_context: ThreadLocal<context::Context>
}

unsafe impl<Mgr: Object<Mgr>> Sync for Shared<Mgr> {}
unsafe impl<Mgr: Object<Mgr>> Send for Shared<Mgr> {}

impl<Mgr: Object<Mgr>> Drop for Shared<Mgr> {
  fn drop(&mut self) {
    let handle = self.worker.lock().unwrap().take();
    if let Some(thrd) = handle {
      nix::unistd::write(self.shutdown_pipe_wr.as_fd(), "UwU".as_bytes()).unwrap();
      
      if thread::current().id() != thrd.thread().id() {
        thrd.join().unwrap();
      }
    }
    
    // Context manager not used anymore, drop it
    unsafe {
      ManuallyDrop::drop(&mut BoxedObject::<Mgr>::from_raw(self.mgr_local_ref.take().unwrap()));
    }
  }
}

pub struct ArcRuntime<Mgr: Object<Mgr>> {
  pub(crate) ____rt: Arc<Shared<Mgr>>
}

impl<Mgr: Object<Mgr>> Clone for ArcRuntime<Mgr> {
  fn clone(&self) -> Self {
    Self {
      ____rt: self.____rt.clone()
    }
  }
}

pub fn new_proxy_manager<B: Into<OwnedFd>>(binder_dev: B) -> Result<ArcRuntime<SelfMananger>, ()> {
  ArcRuntime::new(binder_dev, |_, proxy| Arc::new(SelfMananger(proxy)))
}

impl<Mgr: Object<Mgr>> ArcRuntime<Mgr> {
  pub fn new<F, B: Into<OwnedFd>>(binder_dev: B, manager_proxy_provider: F) -> Result<Self, ()>
    where F: FnOnce(WeakRuntime<Mgr>, Proxy<Mgr>) -> Arc<Mgr>
  {
    Self::new_impl(binder_dev, |weak_rt| {
      manager_proxy_provider(weak_rt.clone(), Proxy::new(weak_rt, CONTEXT_MANAGER_REF))
    })
  }
  
  pub fn downgrade(&self) -> WeakRuntime<Mgr> {
    WeakRuntime {
      ____rt: Arc::downgrade(&self.____rt)
    }
  }
  
  pub fn new_as_manager<F, B: Into<OwnedFd>>(binder_dev: B, manager_provider: F) -> Result<Self, ()>
    where F: FnOnce(WeakRuntime<Mgr>) -> Arc<Mgr>
  {
    let ret = Self::new_impl(binder_dev, manager_provider);
    
    if let Ok(rt) = &ret {
      let mgr_ref = rt.____rt.mgr_local_ref.clone().unwrap();
      libbinder_raw::binder_set_context_mgr(rt.____rt.binder_dev.as_fd(), &mgr_ref).unwrap();
    }
    
    ret
  }
  
  fn new_impl<F, B: Into<OwnedFd>>(binder_dev: B, manager_provider: F) -> Result<Self, ()>
    where F: FnOnce(WeakRuntime<Mgr>) -> Arc<Mgr>
  {
    let binder_dev = Arc::new(binder_dev.into());
    let binder_mem = {
      let len = 8 * 1024 * 1024;
      let ptr = unsafe {
          libc::mmap(ptr::null_mut(),
            len,
            libc::PROT_READ,
            libc::MAP_PRIVATE,
            binder_dev.as_raw_fd(),
            0
          )
        };
      
      OwnedMmap {
        ptr: ptr.cast(),
        len
      }
    };
    
    let ret  = ArcRuntime {
      ____rt: Arc::new_cyclic(|weak| {
        let weak_rt = WeakRuntime { ____rt: weak.clone() };
        let mgr = manager_provider(weak_rt.clone());
        let binder_dev2 = binder_dev.clone();
        
        let (ro, wr) = nix::unistd::pipe().unwrap();
        
        let ro = Arc::new(ro);
        let ro2 = ro.clone();
        Shared {
          mgr_local_ref: Some(unsafe { BoxedObject::new(mgr.clone()).into_raw() }),
          mgr,
          _binder_mem: binder_mem,
          worker: Mutex::new(Some(thread::spawn(move || {
            worker(binder_dev2, weak_rt, ro2)
          }))),
          local_objects_sent_outside: RwLock::new(HashMap::new()),
          shutdown_pipe_wr: wr,
          _shutdown_pipe_ro: ro,
          exec_context: ThreadLocal::new(),
          binder_dev
        }
      })
    };
    Ok(ret)
  }
  
  pub fn get_manager(&self) -> &Arc<Mgr> {
    &self.____rt.mgr
  }
  
  pub fn new_packet<'runtime>(&'runtime self) -> PacketBuilder<'runtime, Mgr> {
    PacketBuilder {
      builder: libbinder_PacketBuilder::new(self.____rt.binder_dev.as_fd()),
      runtime: self
    }
  }
  
  pub fn get_binder<'runtime>(&'runtime self) -> BorrowedFd<'runtime> {
    self.____rt.binder_dev.as_fd()
  }
}

pub struct WeakRuntime<Mgr: Object<Mgr>> {
  ____rt: Weak<Shared<Mgr>>
}

impl<Mgr: Object<Mgr>> Clone for WeakRuntime<Mgr> {
  fn clone(&self) -> Self {
    Self {
      ____rt: self.____rt.clone()
    }
  }
}

impl<Mgr: Object<Mgr>> WeakRuntime<Mgr> {
  pub fn upgrade(&self) -> Option<ArcRuntime<Mgr>> {
    Some(ArcRuntime {
      ____rt: self.____rt.upgrade()?
    })
  }
  
  pub fn ptr_eq(&self, other: &Self) -> bool {
    Weak::ptr_eq(&self.____rt, &other.____rt)
  }
}

