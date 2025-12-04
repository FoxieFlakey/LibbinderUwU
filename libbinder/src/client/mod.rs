use std::{fs::File, os::fd::AsFd};

use libbinder_raw::{ObjectRef, ObjectRefRemote};

use crate::{common::log, packet::builder::PacketBuilder, server};

pub fn init() {
}

pub fn run(binder_dev: &File) {
  log!("Client started");
  log!("Waiting for server");
  server::SERVER_READY.wait_for_completion();
  log!("Server ready");
  
  PacketBuilder::new()
    .set_binder_dev(binder_dev.as_fd())
    .set_target(ObjectRef::Remote(ObjectRefRemote { data_handle: 0 }))
    .set_code(0)
    .build()
    .send();
  
  log!("Client stopped");
}

