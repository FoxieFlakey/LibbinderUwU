use std::{os::fd::{AsFd, OwnedFd}, sync::Arc};

use libbinder::{command_buffer::{Command, CommandBuffer}, return_buffer::ReturnValue};
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};

use crate::{WeakRuntime, context::Context, object::Object};

pub fn worker<Mgr: Object<Mgr> + ?Sized>(binder_dev: Arc<OwnedFd>, weak_rt: WeakRuntime<Mgr>, shutdown_pipe_ro: Arc<OwnedFd>) {
  let ctx = Context::new(binder_dev.as_fd());
  
  let mut cmd_buf = CommandBuffer::new(binder_dev.as_fd());
  cmd_buf.enqueue_command(Command::EnterLooper);
  cmd_buf.exec_always_block(None).unwrap();
  
  loop {
    let mut fds = [
      PollFd::new(binder_dev.as_fd(), PollFlags::POLLIN),
      PollFd::new(shutdown_pipe_ro.as_fd(), PollFlags::POLLIN)
    ];
    
    poll(&mut fds, PollTimeout::NONE).unwrap();
    
    if fds[1].any().unwrap() {
      // Shutdown is requested
      break;
    }
    
    if fds[0].any().unwrap() {
      let Some(rt) = weak_rt.upgrade() else { break; };
      ctx.exec(&rt, |_| {}, |r| {
        match r {
          ReturnValue::TransactionComplete => (),
          _ => ()
        }
      });
    }
    
    let mut cmd_buf = CommandBuffer::new(binder_dev.as_fd());
    cmd_buf.enqueue_command(Command::ExitLooper);
    cmd_buf.exec_always_block(None).unwrap();
  }
}

