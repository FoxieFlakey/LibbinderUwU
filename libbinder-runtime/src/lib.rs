#![feature(ptr_metadata)]
#![feature(unsize)]
#![feature(coerce_unsized)]

// A runtime, for ease of using libbinder
// handles details of thread lifecycle and
// other stuffs

use std::{collections::HashSet, io, marker::PhantomData, mem, os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd}, sync::{Arc, Mutex, OnceLock, RwLock, Weak}, thread::{self, JoinHandle}};

use by_address::ByAddress;
use libbinder::{command_buffer::{Command, CommandBuffer, ExecResult}, packet::PacketSendError, return_buffer::{ReturnBuffer, ReturnValue}};
use libbinder_raw::{binder_set_context_mgr, object::reference::ObjectRefRemote, types::reference::ObjectRef};
use nix::{errno::Errno, fcntl::{OFlag, open}, poll::{PollFd, PollFlags, PollTimeout, poll}, sys::stat::Mode};

use crate::{binder_object::{BinderObject, ConreteObjectFromRemote}, packet::{Packet, PacketBuilder}, proxy::ProxyObject, reference::Reference, util::mmap::{MemorySpan, MmapError, MmapRegion, Protection}};

pub mod binder_object;
pub mod packet;
pub mod proxy;
pub mod reference;
mod util;

struct Shared<ContextManager: BinderObject<ContextManager>> {
  binder_dev: OwnedFd,
  shutdown_pipe_wr: OwnedFd,
  shutdown_pipe_rd: OwnedFd,
  ctx_manager: RwLock<Option<Arc<ContextManager>>>,
  
  // Contains strong references to local objects that was
  // sent out Its used for 'drop' code to remove reference
  // to it.
  //
  // Context manager is not here mainly due its
  // not needed as there strong reference to it
  // exist too on 'ctx_manager' field
  //
  // Because shenanign shit with vtable being differ even
  // for same underlying conrete type, there potentially
  // harmles duplication of live local objects. Kernel will
  // return the exact same data pointer and vtable pointer
  //
  // There also duplication on kernel side too but again harmless
  // just... not worth it to try fix .w. or even impossible
  // so just live with it :(
  local_objects: Mutex<HashSet<ByAddress<Arc<dyn BinderObject<ContextManager>>>>>,
  _binder_buffer: MmapRegion
}

pub struct Runtime<ContextManager: BinderObject<ContextManager>> {
  shared: Arc<Shared<ContextManager>>,
  looper_thrd: OnceLock<JoinHandle<()>>,
  // Exists here, so not contending on the 'ctx_manager' on shared
  // and can be borrowed
  cached_ctx_manager: OnceLock<Arc<ContextManager>>,
  cached_ctx_manager_upcasted: OnceLock<Arc<dyn BinderObject<ContextManager>>>,
  _phantom: PhantomData<&'static ContextManager>
}

impl<ContextManager: BinderObject<ContextManager>> Drop for Runtime<ContextManager> {
  fn drop(&mut self) {
    if let Some(handle) = self.looper_thrd.take() {
      while let Err(e) = nix::unistd::write(self.shared.shutdown_pipe_wr.as_fd(), &[0]) {
        if e != Errno::EINTR {
          panic!("Error writing to shutdown pipe: {e}");
        }
      }
      handle.join().unwrap();
    }
    
    // Remove ref counts
  }
}

pub enum RuntimeCreateError {
  ErrorOpeningBinder(io::Error),
  ErrorCreatingPipe(io::Error),
  ErrorMappingBuffer(io::Error)
}

pub enum RuntimeCreateAsManagerError<ContextManager: BinderObject<ContextManager>> {
  CommonCreateError(RuntimeCreateError),
  CannotBeContextManager(Arc<ContextManager>, io::Error)
}

pub enum RuntimeCreateAsClientError {
  CommonCreateError(RuntimeCreateError),
  WrongContextManagerType
}

impl<ContextManager: BinderObject<ContextManager> + ConreteObjectFromRemote<ContextManager>> Runtime<ContextManager> {
  pub fn new() -> Result<Arc<Self>, RuntimeCreateAsClientError> {
    let rt= Self::new_impl().map_err(RuntimeCreateAsClientError::CommonCreateError)?;
    let concrete_manager = ContextManager::try_from_remote(&rt, ProxyObject { runtime: rt.clone(), remote_ref: ObjectRefRemote { data_handle: 0 } })
      .map(Arc::new)
      .map_err(|_| RuntimeCreateAsClientError::WrongContextManagerType)?;
    *rt.shared.ctx_manager.write().unwrap() = Some(concrete_manager.clone());
    rt.cached_ctx_manager.set(concrete_manager.clone()).ok().unwrap();
    rt.cached_ctx_manager_upcasted.set(concrete_manager).ok().unwrap();
    
    let runtime_weak = Arc::downgrade(&rt);
    rt.looper_thrd.set(thread::spawn(move || {
        run_looper(runtime_weak, false);
      }
    )).unwrap();
    Ok(rt)
  }
}

impl<ContextManager: BinderObject<ContextManager>> Runtime<ContextManager> {
  pub fn new_as_manager(ctx_manager: Arc<ContextManager>) -> Result<Arc<Self>, RuntimeCreateAsManagerError<ContextManager>> {
    let rt = Self::new_impl()
      .map_err(RuntimeCreateAsManagerError::CommonCreateError)?;
    
    let ctx_manager2 = ctx_manager.clone() as Arc<dyn BinderObject<ContextManager>>;
    let local_ref = binder_object::into_local_object_ref(&ctx_manager2);
    if let Err(e) = binder_set_context_mgr(rt.shared.binder_dev.as_fd(), &local_ref) {
      return Err(RuntimeCreateAsManagerError::CannotBeContextManager(ctx_manager, e.into()));
    }
    
    // Note: we manage the reference to concrete mgr
    // as the ctx_manager in the shared
    
    *rt.shared.ctx_manager.write().unwrap() = Some(ctx_manager.clone());
    rt.cached_ctx_manager.set(ctx_manager.clone()).ok().unwrap();
    rt.cached_ctx_manager_upcasted.set(ctx_manager).ok().unwrap();
    
    let runtime_weak = Arc::downgrade(&rt);
    rt.looper_thrd.set(thread::spawn(move || {
        run_looper(runtime_weak, false);
      }
    )).unwrap();
    Ok(rt)
  }
}

impl<ContextManager: BinderObject<ContextManager>> Runtime<ContextManager> {
  pub fn get_context_manager<'a>(&'a self) -> Reference<'a, ContextManager, ContextManager> {
    Reference::context_manager(self)
  }
  
  pub fn get_context_manager_object(&self) -> &Arc<dyn BinderObject<ContextManager>> {
    self.cached_ctx_manager_upcasted.get().unwrap()
  }
}

impl<ContextManager: BinderObject<ContextManager>> Drop for Shared<ContextManager> {
  fn drop(&mut self) {
    for reference in self.local_objects.get_mut().unwrap().drain() {
      let reference = reference.0;
      
      // SAFETY: This is a decrement needed because there +1 reference from kernel
      // if the reference in this hashmap
      unsafe { Arc::decrement_strong_count(Arc::as_ptr(&reference)) };
    }
  }
}

impl<ContextManager: BinderObject<ContextManager>> Runtime<ContextManager> {
  fn new_impl() -> Result<Arc<Self>, RuntimeCreateError> {
    let (rd, wr) = nix::unistd::pipe()
      .map_err(io::Error::from)
      .map_err(RuntimeCreateError::ErrorCreatingPipe)?;
    
    let binder_dev = open("/dev/binder", OFlag::O_RDWR | OFlag::O_CLOEXEC | OFlag::O_NONBLOCK, Mode::empty())
      .map_err(io::Error::from)
      .map_err(RuntimeCreateError::ErrorOpeningBinder)?;
    
    let shared = Arc::new(Shared {
      shutdown_pipe_wr: wr,
      shutdown_pipe_rd: rd,
      local_objects: Mutex::new(HashSet::new()),
      _binder_buffer: MmapRegion::new_map_from_fd(MemorySpan {
          addr: None,
          nr_pages: 512
        }, Protection::Read.into(), false, false, binder_dev.as_fd(), 0)
        .map_err(|x| {
          match x {
            MmapError::MmapError(x) => RuntimeCreateError::ErrorMappingBuffer(x.into())
          }
        })?,
      ctx_manager: RwLock::new(None),
      binder_dev
    });
    
    Ok(Arc::new(Self {
      looper_thrd: OnceLock::new(),
      cached_ctx_manager: OnceLock::new(),
      cached_ctx_manager_upcasted: OnceLock::new(),
      _phantom: PhantomData {},
      shared
    }))
  }
  
  pub(crate) fn get_binder<'a>(&'a self) -> BorrowedFd<'a> {
    self.shared.binder_dev.as_fd()
  }
  
  pub(crate) fn send_packet<'a>(&'a self, target: ObjectRefRemote, packet: &Packet<'a, ContextManager>) -> Result<Packet<'a, ContextManager>, PacketSendError> {
    assert!(self.shared.binder_dev.as_fd().as_raw_fd() == packet.get_binder_dev().as_raw_fd());
    
    // Make sure know all the local objects that was sent outside
    packet.iter_references()
      .inspect(|x| assert!(x.get_binder().as_raw_fd() == self.shared.binder_dev.as_raw_fd(), "attempting to send local object belonging to other runtime"))
      .flat_map(|reference| match reference.get_reference() {
        // Is out problem
        ObjectRef::Local(x) => Some(x.clone()),
        
        // Not our problem
        ObjectRef::Remote(_) => None
      })
      .for_each(|reference| {
        let arc_ref: Arc<dyn BinderObject<ContextManager>> = unsafe { binder_object::from_local_object_ref(&reference) };
        
        let was_succesfully_inserted = self.shared.local_objects.lock()
          .unwrap()
          .insert(ByAddress(arc_ref.clone()));
        
        if was_succesfully_inserted {
          // It doesn't exist, lets leak a reference which means kernel referencing it
          mem::forget(arc_ref);
        }
      });
    
    packet.send(target)
      .map(|packet| Packet::new(self, packet))
  }
  
  pub fn new_packet_builder<'a>(&'a self) -> PacketBuilder<'a, ContextManager> {
    PacketBuilder::new(self)
  }
}

fn run_looper<ContextManager: BinderObject<ContextManager>>(runtime: Weak<Runtime<ContextManager>>, do_register: bool) {
  let shared = match runtime.upgrade() {
    Some(ref rt) => rt.shared.clone(),
    
    // Runtime already dead soo early
    None => return
  };
  
  let binder_dev = shared.binder_dev.as_fd();
  let shutdown_pipe_rd = shared.shutdown_pipe_rd.as_fd();
  let ctx_manager = shared.ctx_manager.read().unwrap().clone().unwrap() as Arc<dyn BinderObject<ContextManager>>;
  
  if do_register {
    CommandBuffer::new(binder_dev)
      .enqueue_command(Command::RegisterLooper)
      .exec_always_block(None)
      .unwrap();
  }
  
  CommandBuffer::new(binder_dev)
    .enqueue_command(Command::EnterLooper)
    .exec_always_block(None)
    .unwrap();
  
  let mut ret_buf = ReturnBuffer::new(binder_dev, 4096);
  'poll_loop: loop {
    let mut fds = [
      PollFd::new(shutdown_pipe_rd.as_fd(), PollFlags::POLLIN),
      PollFd::new(binder_dev, PollFlags::POLLIN),
    ];
    
    loop {
      match poll(&mut fds, PollTimeout::NONE) {
        Ok(_) => break,
        Err(Errno::EINTR) => continue,
        Err(e) => panic!("Error polling: {e}")
      }
    }
    
    if fds[0].any().unwrap() {
      // There something written on the shutdown pipe, quit!
      break 'poll_loop;
    }
    
    if fds[1].any().unwrap() {
      // There incoming data from binder
      match CommandBuffer::new(binder_dev)
        .exec(Some(&mut ret_buf))
        .unwrap()
      {
        // Process whatever data that was received
        ExecResult::Ok | ExecResult::WouldBlockOnRead => (),
        ExecResult::WouldBlockOnWrite(_) => panic!("shouldn't happen")
      }
      
      let Some(runtime) = runtime.upgrade() else {
          // Runtime is dead, quit
          break 'poll_loop;
        };
      
      for v in ret_buf.get_parsed().iter() {
        match v {
          ReturnValue::Noop => (),
          ReturnValue::Transaction((reference, packet)) => {
            let packet = Packet::new(&runtime, packet.clone());
            // SAFETY: Kernel make sure its same pointer as sent
            // which we mem::forget
            let obj = unsafe { binder_object::from_local_object_ref(&reference) };
            let reply = obj.on_packet(&runtime, &packet);
            assert!(reply.get_binder_dev().as_raw_fd() == binder_dev.as_raw_fd(), "Attempt to send reply with packet built for other runtime");
            reply.send_as_reply().unwrap();
            
            // The from_local_object_ref does not increment the counter
            // and don't want to lose reference to it yet
            mem::forget(obj);
          }
          
          ReturnValue::Release(reference) => {
            // SAFETY: Kernel make sure its same pointer as sent
            // which we mem::forget
            let obj = unsafe { binder_object::from_local_object_ref::<ContextManager>(&reference) };
            assert!(Arc::ptr_eq(&obj, &ctx_manager), "BR_RELEASE was trigger for context mananger");
            
            // Remove from local objects list and does not mem::forget the obj to also remove reference
            // from kernel
            assert!(shared.local_objects.lock().unwrap().remove(&ByAddress(obj)), "Kernel sent BR_RELEASE on unknown object");
          }
          
          ReturnValue::Acquire(_) |
          ReturnValue::AcquireWeak(_) |
          ReturnValue::ReleaseWeak(_) |
          ReturnValue::Ok |
          ReturnValue::Error(_) |
          ReturnValue::SpawnLooper |
          ReturnValue::DeadReply |
          ReturnValue::TransactionComplete |
          ReturnValue::TransactionFailed |
          ReturnValue::Reply(_)
          => unimplemented!()
        }
      };
    }
  }
  
  CommandBuffer::new(binder_dev)
    .enqueue_command(Command::ExitLooper)
    .exec_always_block(None)
    .unwrap();
}



