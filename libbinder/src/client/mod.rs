use std::{fs::File, os::fd::AsFd};

use libbinder_raw::{ObjectRef, ObjectRefRemote};
use nix::unistd::sleep;

use crate::{common::log, packet::builder::PacketBuilder, server};

pub fn init() {
}

pub fn run(binder_dev: &File) {
  log!("Client started");
  log!("Waiting for server");
  server::SERVER_READY.wait_for_completion();
  sleep(1);
  log!("Server ready");
  
  PacketBuilder::new()
    .set_binder_dev(binder_dev.as_fd())
    .set_code(0)
    .build()
    .send(ObjectRef::Remote(ObjectRefRemote { data_handle: 0 }));
  
  log!("Client stopped");
}

