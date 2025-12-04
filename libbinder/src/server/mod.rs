use std::{fs::File, os::fd::AsFd};

use libbinder_raw::{ObjectRefLocal, binder_set_context_mgr};

use crate::common::log;

pub fn init() {
}

pub fn run(binder_dev: &File) {
  let ctx_mgr = ObjectRefLocal {
    data: 0,
    extra_data: 0,
  };
  
  binder_set_context_mgr(binder_dev.as_fd(), &ctx_mgr).unwrap();
  log!("Server started");
  log!("Server stopped");
}


