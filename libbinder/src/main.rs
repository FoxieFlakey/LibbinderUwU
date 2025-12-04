#![feature(never_type)]

use std::{fmt::Write, fs::File, os::fd::{AsFd, AsRawFd}, process::exit, ptr};

use libbinder_raw::{BINDER_COMPILED_VERSION, binder_version};
use nix::{libc, sys::wait::waitpid, unistd::{ForkResult, Pid, fork}};

mod packet;
mod process_sync;
mod server;
mod client;
mod common;
mod command_buffer;
mod return_buffer;

use common::log;

pub fn hexdump(bytes: &[u8]) {
  let (chunks, remainder) = bytes.as_chunks::<32>();
  fn dump(bytes: &[u8]) {
    let mut serialized = String::new();
    for byte in bytes {
      write!(&mut serialized, "{byte:02x} ").unwrap();
    }
    serialized.pop();
    println!("0x{:#16x} {serialized}", bytes.as_ptr().addr());
  }
  
  chunks.iter()
    .for_each(|x| dump(x));
  dump(remainder);
}

fn divide<F: FnOnce()>(on_child: F) -> Pid {
  match unsafe { fork() }.unwrap() {
    ForkResult::Child => {
      on_child();
      exit(0);
    },
    ForkResult::Parent { child } => child
  }
}

fn common_prep_binder() -> File {
  let binder = File::open("/dev/binder").unwrap();
  let fd = binder.as_raw_fd();
  // Binder need this for storing transactions buffer
  let ret = unsafe {
    libc::mmap(
      ptr::null_mut(),
      8 * 1024 * 1024,
      libc::PROT_READ,
      libc::MAP_PRIVATE,
      fd,
      0
    )
  };
  
  assert!(ret != libc::MAP_FAILED);
  binder
}

fn main() {
  let binder_dev = File::open("/dev/binder").unwrap();
  let ver = binder_version(binder_dev.as_fd()).unwrap();
  println!("Binder version on kernel is {} while libkernel compiled for {}", ver.version, BINDER_COMPILED_VERSION.version);
  
  client::init();
  server::init();
  log!("Inited");
  
  let server_pid = divide(|| server::run(&common_prep_binder()));
  let child_pid = divide(|| client::run(&common_prep_binder()));
  
  log!("Waiting");
  
  waitpid(server_pid, None).unwrap();
  waitpid(child_pid, None).unwrap();
  
  log!("Done");
}
