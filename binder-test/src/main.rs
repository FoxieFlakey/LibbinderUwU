#![feature(never_type)]

use std::{fmt::Write, fs::File, process::exit, sync::Arc, thread, time::Duration};

use libbinder_runtime::{ArcRuntime, object::Object, packet::dead_simple::DeadSimpleFormat};
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
      struct ContextManager;
      
      impl Object<ContextManager> for ContextManager {
        fn do_transaction<'runtime>(&self, packet: &'runtime libbinder_runtime::packet::Packet<'runtime, ContextManager>) -> Result<libbinder_runtime::packet::Packet<'runtime, ContextManager>, libbinder_runtime::object::TransactionError> {
          panic!("TODO");
        }
      }
      
      let rt = ArcRuntime::new_as_manager(binder_dev, |weak_rt| {
        Arc::new(ContextManager)
      });
      
      thread::sleep(Duration::from_secs(3));
    }),
    divide(|| {
      thread::sleep(Duration::from_secs(1));
      
      let binder_dev = File::open("/dev/binder").unwrap();
      let rt = libbinder_runtime::new_proxy_manager(binder_dev).unwrap();
      let mut packet = rt.new_packet();
      packet
        .set_code(0)
        .writer(DeadSimpleFormat::new())
        .write_str("Hello World!")
        .write_bool(true);
      
      rt.get_manager().do_transaction(&packet.build()).unwrap();
    })
  ].iter().for_each(|pid| {
    waitpid(*pid, None).unwrap();
  });
}

