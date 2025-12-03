use std::{fs::File, os::fd::AsFd};

use libbinder_raw::{BINDER_COMPILED_VERSION, ObjectRefLocal, binder_set_context_mgr, binder_version};

fn main() {
  let binder_dev = File::open("/dev/binder").unwrap();
  let ver = binder_version(binder_dev.as_fd()).unwrap();
  println!("Binder version on kernel is {} while libkernel compiled for {}", ver.version, BINDER_COMPILED_VERSION.version);
  
  let context_mgr = ObjectRefLocal {
    data: 0,
    extra_data: 0
  };
  binder_set_context_mgr(binder_dev.as_fd(), &context_mgr).unwrap();
}


