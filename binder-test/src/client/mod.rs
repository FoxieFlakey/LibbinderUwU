use std::{fs::File, os::fd::AsFd};

use libbinder::packet::builder::PacketBuilder;
use libbinder_raw::{ObjectRef, ObjectRefRemote};
use nix::unistd::sleep;

use crate::{common::log, server};

pub fn init() {
}

pub fn run(binder_dev: &File) {
  log!("Client started");
  log!("Waiting for server");
  server::SERVER_READY.wait_for_completion();
  sleep(1);
  log!("Server ready");
  
  PacketBuilder::new(binder_dev.as_fd())
    .set_code(8086)
    .build()
    .send(ObjectRef::Remote(ObjectRefRemote { data_handle: 0 }))
    .unwrap();
  
  log!("Client stopped");
}

