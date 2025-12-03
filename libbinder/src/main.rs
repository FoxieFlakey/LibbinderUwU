use std::{fs::File, os::fd::AsFd};

use libbinder_raw::{BINDER_COMPILED_VERSION, Command, ObjectRefLocal, binder_read_write, binder_set_context_mgr, binder_version};

fn main() {
  let binder_dev = File::open("/dev/binder").unwrap();
  let ver = binder_version(binder_dev.as_fd()).unwrap();
  println!("Binder version on kernel is {} while libkernel compiled for {}", ver.version, BINDER_COMPILED_VERSION.version);
  
  let context_mgr = ObjectRefLocal {
    data: 0,
    extra_data: 0
  };
  binder_set_context_mgr(binder_dev.as_fd(), &context_mgr).unwrap();
  
  let mut commands = Vec::<u8>::new();
  let mut response = Vec::<u8>::new();
  response.reserve(100);
  commands.extend_from_slice(&Command::EnterLooper.as_bytes());
  commands.extend_from_slice(&Command::ExitLooper.as_bytes());
  
  let (_, written_in_read) = binder_read_write(binder_dev.as_fd(), &commands, &mut response).unwrap();
  let (ret_aligned, remainder) = response[..written_in_read].as_chunks::<4>();
  println!("Number of return values {} remainer bytes {}", ret_aligned.len(), remainder.len());
}


