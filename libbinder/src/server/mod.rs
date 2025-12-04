use std::{fs::File, os::fd::AsFd, sync::LazyLock};

use libbinder_raw::{Command, ObjectRefLocal, binder_read_write, binder_set_context_mgr};

use crate::{common::log, hexdump, process_sync::shared_completion::SharedCompletion};

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
  
  let mut write_buf = Vec::new();
  let mut read_buf = Vec::new();
  read_buf.resize(4096, 0u8);
  
  write_buf.extend_from_slice(&Command::EnterLooper.as_bytes());
  let (bytes_written, bytes_read) = binder_read_write(binder_dev.as_fd(), &write_buf, &mut read_buf).unwrap();
  println!("Bytes written: {bytes_written} Bytes read: {bytes_read}");
  hexdump(&read_buf[..bytes_read]);
  
  let (_, bytes_read) = binder_read_write(binder_dev.as_fd(), &[], &mut read_buf).unwrap();
  println!("Bytes written: {bytes_written} Bytes read: {bytes_read}");
  hexdump(&read_buf[..bytes_read]);
  
  log!("Server stopped");
}


