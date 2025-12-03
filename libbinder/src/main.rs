use std::{fmt::Write, fs::File, os::fd::AsFd};

use libbinder_raw::{BINDER_COMPILED_VERSION, Command, ObjectRefLocal, TransactionDataCommon, TransactionToKernel, binder_read_write, binder_set_context_mgr, binder_version};

fn hexdump(bytes: &[u8]) {
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
  
  let mut commands = Vec::<u8>::new();
  let mut response = Vec::<u8>::new();
  response.resize_with(100, || 0);
  commands.extend_from_slice(&Command::EnterLooper.as_bytes());
  commands.extend_from_slice(&Command::ExitLooper.as_bytes());
  
  let data = TransactionToKernel {
    data: TransactionDataCommon {
      code: 0,
      data_slice: &[],
      offsets: &[]
    },
    target: 0
  };
  
  data.with_bytes(|bytes| {
    commands.extend_from_slice(&Command::SendTransaction.as_bytes());
    commands.extend_from_slice(bytes);
  });
  
  let (_, written_in_read) = binder_read_write(binder_dev.as_fd(), &commands, &mut response).unwrap();
  println!("Bytes read: {written_in_read}");
  hexdump(&response[..written_in_read]);
  
  drop(data);
}


