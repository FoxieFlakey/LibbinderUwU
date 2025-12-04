#![feature(never_type)]

use std::{fmt::Write, fs::File, os::fd::AsFd, process::exit};

use libbinder_raw::{BINDER_COMPILED_VERSION, binder_version};
use nix::{sys::wait::waitpid, unistd::{ForkResult, Pid, fork}};

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

fn main() {
  let binder_dev = File::open("/dev/binder").unwrap();
  let ver = binder_version(binder_dev.as_fd()).unwrap();
  println!("Binder version on kernel is {} while libkernel compiled for {}", ver.version, BINDER_COMPILED_VERSION.version);
  
  client::init();
  server::init();
  log!("Inited");
  
  let server_pid = divide(|| server::run(&File::open("/dev/binder").unwrap()));
  let child_pid = divide(|| client::run(&File::open("/dev/binder").unwrap()));
  
  log!("Waiting");
  
  waitpid(server_pid, None).unwrap();
  waitpid(child_pid, None).unwrap();
  
  log!("Done");
}
