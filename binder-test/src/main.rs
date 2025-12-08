#![feature(never_type)]

use std::{fmt::Write, fs::File, os::fd::AsFd, process::exit};

use libbinder_runtime::{BINDER_COMPILED_VERSION, binder_version};
use nix::{sys::wait::waitpid, unistd::{ForkResult, Pid, fork}};

mod common;
mod service_manager;
mod process_sync;
mod app;
mod interface;

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

fn main() {
  let binder_dev = File::open("/dev/binder").unwrap();
  let ver = binder_version(binder_dev.as_fd()).unwrap();
  println!("Binder version on kernel is {} while libkernel compiled for {}", ver.version, BINDER_COMPILED_VERSION.version);
  
  [
    divide(|| service_manager::main()),
    divide(|| app::main())
  ].iter().for_each(|pid| {
    waitpid(*pid, None).unwrap();
  });
}

