use std::{fs::File, os::fd::AsFd, sync::LazyLock};

use libbinder_raw::{ObjectRefLocal, binder_set_context_mgr};

use crate::{command_buffer::{Command, CommandBuffer}, common::log, process_sync::shared_completion::SharedCompletion, return_buffer::ReturnBuffer};

pub static SERVER_READY: LazyLock<SharedCompletion> = LazyLock::new(SharedCompletion::new);

pub fn init() {
  LazyLock::force(&SERVER_READY);
}

pub fn run(binder_dev: &File) {
  let ctx_mgr = ObjectRefLocal {
    data: 0,
    extra_data: 0,
  };
  
  binder_set_context_mgr(binder_dev.as_fd(), &ctx_mgr).unwrap();
  
  log!("Server started");
  SERVER_READY.complete();
  
  CommandBuffer::new(binder_dev.as_fd())
    .enqueue_command(Command::EnterLooper)
    .exec(None);
  
  let mut ret_buf = ReturnBuffer::new(binder_dev.as_fd(), 4096);
  CommandBuffer::new(binder_dev.as_fd()).exec(Some(&mut ret_buf));
  log!("Server stopped");
}


