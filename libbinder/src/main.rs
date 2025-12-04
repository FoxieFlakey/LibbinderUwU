use std::{fmt::Write, fs::File, os::fd::AsFd};

use enumflags2::BitFlags;
use libbinder_raw::{BINDER_COMPILED_VERSION, ObjectRef, ObjectRefLocal, ObjectRefRemote, binder_set_context_mgr, binder_version};

use crate::packet::builder::PacketBuilder;

mod packet;

pub fn hexdump(bytes: &[u8]) {
  let (chunks, remainder) = bytes.as_chunks::<64>();
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

fn main() {
  let binder_dev = File::open("/dev/binder").unwrap();
  let ver = binder_version(binder_dev.as_fd()).unwrap();
  println!("Binder version on kernel is {} while libkernel compiled for {}", ver.version, BINDER_COMPILED_VERSION.version);
  
  let context_mgr = ObjectRefLocal {
    data: 0,
    extra_data: 0
  };
  binder_set_context_mgr(binder_dev.as_fd(), &context_mgr).unwrap();
  
  PacketBuilder::new()
    .set_binder_dev(binder_dev.as_fd())
    .set_code(0)
    .set_flags(BitFlags::empty())
    .set_target(ObjectRef::Remote(ObjectRefRemote { data_handle: 0 }))
    .build()
    .send();
}
