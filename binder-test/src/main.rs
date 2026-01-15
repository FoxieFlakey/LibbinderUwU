#![feature(never_type)]

use std::{fmt::Write, fs::File, process::exit, sync::Arc, thread, time::Duration};

use libbinder_runtime::{ArcRuntime, WeakRuntime, object::Object, packet::dead_simple::{DeadSimpleFormat, DeadSimpleFormatReader}};
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
      struct ContextManager {
        weak_rt: WeakRuntime<ContextManager>
      }
      
      impl Object<ContextManager> for ContextManager {
        fn do_transaction<'packet, 'runtime>(&self, packet: &'packet libbinder_runtime::packet::Packet<'runtime, ContextManager>) -> Result<libbinder_runtime::packet::Packet<'runtime, ContextManager>, libbinder_runtime::object::TransactionError> {
          let rt = packet.get_runtime();
          assert!(rt.downgrade().ptr_eq(&self.weak_rt), "attempt to do transaction belonging other runtime");
          
          let mut builder = rt
            .new_packet();
          
          builder.set_code(0)
            .writer(DeadSimpleFormat::new())
            .write_str("Hello World UwU");
          
          Ok(builder.build())
        }
      }
      
      let _rt = ArcRuntime::new_as_manager(binder_dev, |weak_rt| {
        Arc::new(ContextManager { weak_rt })
      });
      
      thread::sleep(Duration::from_secs(5));
    }),
    divide(|| {
      thread::sleep(Duration::from_secs(2));
      
      let binder_dev = File::open("/dev/binder").unwrap();
      let rt = libbinder_runtime::new_proxy_manager(binder_dev).unwrap();
      let mut packet = rt.new_packet();
      packet
        .set_code(0)
        .writer(DeadSimpleFormat::new())
        .write_str("Hello World!")
        .write_bool(true);
      
      let response = rt.get_manager().do_transaction(&packet.build()).unwrap();
      let response = response.reader(DeadSimpleFormatReader::new())
        .read_str()
        .unwrap();
      println!("Response '{response}'");
    })
  ].iter().for_each(|pid| {
    waitpid(*pid, None).unwrap();
  });
}

