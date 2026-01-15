#![feature(ptr_metadata)]

use std::{collections::HashMap, os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd}, ptr, sync::{Arc, Mutex, RwLock, Weak, atomic::AtomicU64}, thread::{self, JoinHandle}};

use libbinder::command_buffer::{Command, CommandBuffer};
use libbinder_raw::types::reference::{CONTEXT_MANAGER_REF, ObjectRefLocal, ObjectRefRemote};
use nix::libc;
use thread_local::ThreadLocal;

use crate::{object::Object, packet::builder::PacketBuilder, proxy::{Proxy, SelfMananger}, util::OwnedMmap, worker::worker};

pub mod object;
pub mod packet;
pub mod proxy;
pub mod reference;

mod util;
mod worker;
mod context;

pub(crate) struct Shared<Mgr: Object<Mgr>> {
  pub(crate) binder_dev: Arc<OwnedFd>,
  mgr: RwLock<(Option<Arc<Mgr>>, Option<ObjectRefLocal>)>,
  
  // Used by Binder to store incoming transaction and buffer :3
  // don't need to be used
  _binder_mem: OwnedMmap,
  shutdown_pipe_wr: OwnedFd,
  _shutdown_pipe_ro: Arc<OwnedFd>,
  worker: Mutex<Option<JoinHandle<()>>>,
  
  // .0 is there any strong reference from outside
  // .1 is there any weak reference from outside
  // .0 and .1 will never be false at same time
  reference_states: Mutex<HashMap<ObjectRefLocal, (bool, bool)>>,
  
  // The ref count here may be touched to 0 when only read lock
  // is taken, check again when upgrade to write lock
  remote_reference_counters: RwLock<HashMap<ObjectRefRemote, AtomicU64>>,
  
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
    
    for (&local_ref, _) in self.reference_states.get_mut().unwrap().iter() {
      // Remove all currently exist references
      drop(unsafe { object::from_local_ref::<Mgr>(local_ref) });
    }
    
    let mut buf = CommandBuffer::new(self.binder_dev.as_fd());
    for (&remote_ref, counter) in self.remote_reference_counters.get_mut().unwrap().iter_mut() {
      if *counter.get_mut() == 0 {
        // There was stale reference inside
        continue;
      }
      
      buf = buf.clear();
      buf.enqueue_command(Command::Release(remote_ref));
      buf.exec_always_block(None).unwrap();
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
  ArcRuntime::new(binder_dev, |_, proxy| SelfMananger(proxy))
}

impl<Mgr: Object<Mgr>> ArcRuntime<Mgr> {
  pub fn new<F, B: Into<OwnedFd>>(binder_dev: B, manager_proxy_provider: F) -> Result<Self, ()>
    where F: FnOnce(ArcRuntime<Mgr>, Proxy<Mgr>) -> Mgr
  {
    let rt = Self::new_impl(binder_dev)?;
    let mgr = Arc::new(manager_proxy_provider(rt.clone(), Proxy::new(rt.downgrade(), CONTEXT_MANAGER_REF)));
    *rt.____rt.mgr.write().unwrap() = (Some(mgr), None);
    Ok(rt)
  }
  
  pub fn downgrade(&self) -> WeakRuntime<Mgr> {
    WeakRuntime {
      ____rt: Arc::downgrade(&self.____rt)
    }
  }
  
  pub fn new_as_manager<F, B: Into<OwnedFd>>(binder_dev: B, manager_provider: F) -> Result<Self, ()>
    where F: FnOnce(ArcRuntime<Mgr>) -> Mgr
  {
    let rt = Self::new_impl(binder_dev)?;
    let mgr = Arc::new(manager_provider(rt.clone()));
    let mgr_ref = object::into_local_ref(mgr.clone());
    *rt.____rt.mgr.write().unwrap() = (Some(mgr), Some(mgr_ref));
    rt.____rt.reference_states.lock().unwrap().insert(mgr_ref, (true, false));
    
    libbinder_raw::binder_set_context_mgr(rt.____rt.binder_dev.as_fd(), &mgr_ref).unwrap();
    
    Ok(rt)
  }
  
  fn new_impl<B: Into<OwnedFd>>(binder_dev: B) -> Result<Self, ()> {
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
        let binder_dev2 = binder_dev.clone();
        
        let (ro, wr) = nix::unistd::pipe().unwrap();
        
        let ro = Arc::new(ro);
        let ro2 = ro.clone();
        
        Shared {
          mgr: RwLock::new((None, None)),
          _binder_mem: binder_mem,
          worker: Mutex::new(Some(thread::spawn(move || {
            worker(binder_dev2, weak_rt, ro2)
          }))),
          reference_states: Mutex::new(HashMap::new()),
          remote_reference_counters: RwLock::new(HashMap::new()),
          shutdown_pipe_wr: wr,
          _shutdown_pipe_ro: ro,
          exec_context: ThreadLocal::new(),
          binder_dev
        }
      })
    };
    Ok(ret)
  }
  
  pub fn get_manager(&self) -> Arc<Mgr> {
    self.____rt.mgr.read().unwrap().0.clone().unwrap()
  }
  
  pub fn ptr_eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.____rt, &other.____rt)
  }
  
  pub fn new_packet<'runtime>(&'runtime self) -> PacketBuilder<'runtime, Mgr> {
    PacketBuilder::new(self)
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

