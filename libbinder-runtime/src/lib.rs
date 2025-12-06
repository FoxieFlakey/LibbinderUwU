#![feature(ptr_metadata)]

// A runtime, for ease of using libbinder
// handles details of thread lifecycle and
// other stuffs

use std::{io, marker::PhantomData, os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd}, sync::{Arc, RwLock}, thread::{self, JoinHandle}};

use closure::closure;
use libbinder::{command_buffer::{Command, CommandBuffer, ExecResult}, packet::{Packet, PacketSendError, builder::PacketBuilder}, return_buffer::{ReturnBuffer, ReturnValue}};
use libbinder_raw::{ObjectRefRemote, binder_set_context_mgr};
use nix::{errno::Errno, fcntl::{OFlag, open}, poll::{PollFd, PollFlags, PollTimeout, poll}, sys::stat::Mode};

use crate::{binder_object::{BinderObject, ConreteObjectFromRemote}, util::mmap::{MemorySpan, MmapError, MmapRegion, Protection}};

pub mod binder_object;
mod util;

struct Shared {
  binder_dev: OwnedFd,
  shutdown_pipe_wr: OwnedFd,
  ctx_manager: RwLock<Option<Arc<dyn BinderObject>>>,
  _binder_buffer: MmapRegion
}

pub struct Runtime<ContextManager: BinderObject + ?Sized> {
  shared: Arc<Shared>,
  looper_thrd: Option<JoinHandle<()>>,
  _phantom: PhantomData<&'static ContextManager>
}

impl<ContextManager: BinderObject + ?Sized> Drop for Runtime<ContextManager> {
  fn drop(&mut self) {
    if let Some(handle) = self.looper_thrd.take() {
      while let Err(e) = nix::unistd::write(self.shared.shutdown_pipe_wr.as_fd(), &[0]) {
        if e != Errno::EINTR {
          panic!("Error writing to shutdown pipe: {e}");
        }
      }
      handle.join().unwrap();
    }
  }
}

pub enum RuntimeCreateError {
  ErrorOpeningBinder(io::Error),
  ErrorCreatingPipe(io::Error),
  ErrorMappingBuffer(io::Error)
}

pub enum RuntimeCreateAsManagerError<ContextManager: BinderObject> {
  CommonCreateError(RuntimeCreateError),
  CannotBeContextManager(Arc<ContextManager>, io::Error)
}

pub enum RuntimeCreateAsClientError {
  CommonCreateError(RuntimeCreateError),
  WrongContextManagerType
}

impl<ContextManager: BinderObject + ConreteObjectFromRemote<ContextManager>> Runtime<ContextManager> {
  pub fn new() -> Result<Self, RuntimeCreateAsClientError> {
    let rt= Self::new_impl().map_err(RuntimeCreateAsClientError::CommonCreateError)?;
    let concrete_manager = ContextManager::try_from_remote(&rt, ObjectRefRemote { data_handle: 0 })
      .map(Arc::new)
      .map_err(|_| RuntimeCreateAsClientError::WrongContextManagerType)?;
    *rt.shared.ctx_manager.write().unwrap() = Some(concrete_manager);
    Ok(rt)
  }
}

impl<ContextManager: BinderObject> Runtime<ContextManager> {
  pub fn new_as_manager(ctx_manager: Arc<ContextManager>) -> Result<Self, RuntimeCreateAsManagerError<ContextManager>> {
    let ret = Self::new_impl()
      .map_err(RuntimeCreateAsManagerError::CommonCreateError)?;
    
    let ctx_manager2 = ctx_manager.clone() as Arc<dyn BinderObject>;
    let local_ref = binder_object::into_local_object_ref(&ctx_manager2);
    if let Err(e) = binder_set_context_mgr(ret.shared.binder_dev.as_fd(), &local_ref) {
      return Err(RuntimeCreateAsManagerError::CannotBeContextManager(ctx_manager, e.into()));
    }
    
    // Note: we manage the reference to concrete mgr
    // as the ctx_manager in the shared
    
    *ret.shared.ctx_manager.write().unwrap() = Some(ctx_manager2);
    Ok(ret)
  }
}

impl<ContextManager: BinderObject> Runtime<ContextManager> {
  pub fn get_context_manager(&self) -> Arc<ContextManager> {
    Arc::downcast(self.get_context_manager_object().clone()).unwrap()
  }
}

impl<ContextManager: BinderObject + ?Sized> Runtime<ContextManager> {
  pub fn get_context_manager_object(&self) -> Arc<dyn BinderObject> {
    self.shared.ctx_manager.read().unwrap().clone().unwrap()
  }
}

impl<ContextManager: BinderObject + ?Sized> Runtime<ContextManager> {
  fn new_impl() -> Result<Self, RuntimeCreateError> {
    let (rd, wr) = nix::unistd::pipe()
      .map_err(io::Error::from)
      .map_err(RuntimeCreateError::ErrorCreatingPipe)?;
    
    let binder_dev = open("/dev/binder", OFlag::O_RDWR | OFlag::O_CLOEXEC | OFlag::O_NONBLOCK, Mode::empty())
      .map_err(io::Error::from)
      .map_err(RuntimeCreateError::ErrorOpeningBinder)?;
    
    let shared = Arc::new(Shared {
      shutdown_pipe_wr: wr,
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
    
    Ok(Self {
      looper_thrd: Some(thread::spawn(closure!(
        clone shared,
        move rd,
        || {
          run_looper(shared, rd, false);
        }
      ))),
      _phantom: PhantomData {},
      shared
    })
  }
  
  pub fn get_binder<'a>(&'a self) -> BorrowedFd<'a> {
    self.shared.binder_dev.as_fd()
  }
  
  pub fn send_packet<'a>(&'a self, target: ObjectRefRemote, packet: &Packet<'a>) -> Result<Packet<'a>, PacketSendError> {
    assert!(self.shared.binder_dev.as_fd().as_raw_fd() == packet.get_binder_dev().as_raw_fd());
    packet.send(target)
  }
}

fn run_looper(shared: Arc<Shared>, shutdown_pipe_rd: OwnedFd, do_register: bool) {
  let binder_dev = shared.binder_dev.as_fd();
  
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
  let mut reply_builder = PacketBuilder::new();
  'poll_loop: loop {
    let mut fds = [
      PollFd::new(shutdown_pipe_rd.as_fd(), PollFlags::POLLIN),
      PollFd::new(binder_dev, PollFlags::POLLIN),
    ];
    
    let mock_runtime = Runtime {
      shared: shared.clone(),
      _phantom: PhantomData {},
      looper_thrd: None
    };
    
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
      
      ret_buf.get_parsed()
        .iter()
        .for_each(|v| {
          match v {
            ReturnValue::Noop => (),
            ReturnValue::Transaction((reference, packet)) => {
              let obj = unsafe { binder_object::from_local_object_ref(&reference) };
              
              obj.on_packet(&mock_runtime, &packet, &mut reply_builder);
              
              let reply = reply_builder.build(binder_dev);
              reply.send_as_reply().unwrap();
              reply_builder = reply.into();
            }
            _ => unimplemented!()
          }
        });
    }
  }
  
  CommandBuffer::new(binder_dev)
    .enqueue_command(Command::ExitLooper)
    .exec_always_block(None)
    .unwrap();
}



