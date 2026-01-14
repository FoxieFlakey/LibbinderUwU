#![feature(never_type)]

use std::{fmt::Write, fs::File, process::exit};

use libbinder_runtime::{object::Object, packet::dead_simple::DeadSimpleFormat};
use nix::{sys::wait::waitpid, unistd::{ForkResult, Pid, fork}};

mod common;
mod process_sync;

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
  [
    divide(|| {
      let binder_dev = File::open("/dev/binder").unwrap();
      let rt = libbinder_runtime::new_proxy_manager(binder_dev).unwrap();
      let mut packet = rt.new_packet();
      packet
        .set_code(0)
        .writer(DeadSimpleFormat::new())
        .write_str("Hello World!")
        .write_bool(true);
      
      let _ = rt.get_manager().on_packet(&packet.build());
    })
  ].iter().for_each(|pid| {
    waitpid(*pid, None).unwrap();
  });
}

