use std::{os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd}, ptr, sync::{Arc, Weak}};

use libbinder::packet::builder::PacketBuilder as libbinder_PacketBuilder;
use libbinder_raw::types::reference::CONTEXT_MANAGER_REF;
use nix::libc;

use crate::{object::Object, packet::builder::PacketBuilder, proxy::{Proxy, SelfMananger}, util::OwnedMmap};

pub mod object;
pub mod packet;
pub mod proxy;

mod util;

pub(crate) struct Shared<Mgr: Object<Mgr>> {
  binder_dev: Arc<OwnedFd>,
  mgr: Arc<Mgr>,
  
  // Used by Binder to store incoming transaction and buffer :3
  // don't need to be used
  _binder_mem: OwnedMmap
}

#[derive(Clone)]
pub struct ArcRuntime<Mgr: Object<Mgr>> {
  ____rt: Arc<Shared<Mgr>>
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
    
    Ok(ArcRuntime {
      ____rt: Arc::new_cyclic(|weak| {
        let weak_rt = WeakRuntime { ____rt: weak.clone() };
        
        Shared {
          mgr: manager_provider(weak_rt),
          _binder_mem: binder_mem,
          binder_dev
        }
      })
    })
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

