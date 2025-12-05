#![feature(ptr_metadata)]

// A runtime, for ease of using libbinder
// handles details of thread lifecycle and
// other stuffs

use std::{io, os::fd::{AsFd, AsRawFd, OwnedFd}, sync::Arc, thread::{self, JoinHandle}};

use closure::closure;
use libbinder::{command_buffer::{Command, CommandBuffer, ExecResult}, packet::{Packet, PacketSendError, builder::PacketBuilder}, return_buffer::{ReturnBuffer, ReturnValue}};
use libbinder_raw::{ObjectRefRemote, binder_set_context_mgr};
use nix::{errno::Errno, fcntl::{OFlag, open}, poll::{PollFd, PollFlags, PollTimeout, poll}, sys::stat::Mode};

use crate::{binder_object::BinderObject, util::mmap::{MemorySpan, MmapError, MmapRegion, Protection}};

pub mod binder_object;
mod util;

struct Shared {
  binder_dev: OwnedFd,
  shutdown_pipe_wr: OwnedFd,
  _binder_buffer: MmapRegion
}

pub struct Runtime {
  shared: Arc<Shared>,
  looper_thrd: Option<JoinHandle<()>>
}

impl Drop for Runtime {
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

#[derive(Debug)]
pub enum RuntimeCreateError {
  ErrorOpeningBinder(io::Error),
  ErrorCreatingPipe(io::Error),
  ErrorMappingBuffer(io::Error)
}

impl Runtime {
  pub fn new() -> Result<Self, RuntimeCreateError> {
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
      shared
    })
  }
  
  pub fn new_packet<'a>(&'a self) -> PacketBuilder<'a> {
    PacketBuilder::new(self.shared.binder_dev.as_fd())
  }
  
  pub fn send_packet<'a>(&'a self, target: ObjectRefRemote, packet: &Packet<'a>) -> Result<Packet<'a>, PacketSendError> {
    assert!(self.shared.binder_dev.as_fd().as_raw_fd() == packet.get_binder_dev().as_raw_fd());
    packet.send(target)
  }
  
  pub fn become_manager(&self, mgr_object: Box<dyn BinderObject>) -> Result<(), (Box<dyn BinderObject>, io::Error)> {
    // TODO: Handle memory allocation instead leaking
    let leaked = Box::leak(mgr_object);
    let local_ref = binder_object::into_local_object_ref(leaked);
    if let Err(e) = binder_set_context_mgr(self.shared.binder_dev.as_fd(), &local_ref) {
      // SAFETY: We made it with Box
      return Err((unsafe { Box::from_raw(leaked) }, e.into()));
    }
    Ok(())
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
  let mut reply_builder = PacketBuilder::new(shared.binder_dev.as_fd());
  'poll_loop: loop {
    let mut fds = [
      PollFd::new(shutdown_pipe_rd.as_fd(), PollFlags::POLLIN),
      PollFd::new(binder_dev, PollFlags::POLLIN),
    ];
    
    let mock_runtime = Runtime {
      shared: shared.clone(),
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
              let obj = unsafe {
                binder_object::from_local_object_ref(&reference)
                  .as_ref()
                  .unwrap()
              };
              
              obj.on_packet(&mock_runtime, &packet, &mut reply_builder);
              
              let reply = reply_builder.build();
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



