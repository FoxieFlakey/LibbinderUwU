use std::{fs::File, os::fd::AsFd, sync::LazyLock};

use libbinder_raw::{ObjectRefLocal, binder_set_context_mgr};

use libbinder::{command_buffer::{Command, CommandBuffer}, return_buffer::{ReturnBuffer, ReturnValue}};
use crate::{common::log, process_sync::shared_completion::SharedCompletion};

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
  ret_buf.get_parsed()
    .iter()
    .rev()
    .for_each(|ret| {
      if let ReturnValue::Transaction((_, packet)) = ret {
        log!("Incoming transaction! code {}", packet.get_code());
        let mut response = packet.clone();
        response.set_code(80386);
        response.send_as_reply();
      } else if !matches!(ret, ReturnValue::Noop) {
        panic!("Unknown");
      }
    });
  
  log!("Server stopped");
}


