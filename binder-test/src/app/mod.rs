use libbinder::formats::dead_simple::DeadSimpleFormatReader;
use libbinder_raw::{ObjectRef, ObjectRefRemote};
use libbinder_runtime::Runtime;

use crate::common::log;

pub fn main() {
  let runtime = Runtime::new().unwrap();
  nix::unistd::sleep(1);
  
  let response = runtime.send_packet(
      ObjectRef::Remote(ObjectRefRemote { data_handle: 0 }),
      &runtime.new_packet()
        .set_code(80386)
        .build()
    ).unwrap();
  
  assert!(response.get_code() == 7875);
  
  let mut reader = response.reader(DeadSimpleFormatReader::new());
  log!("Read f64: {}", reader.read_f64().unwrap());
  log!("Read f32: {}", reader.read_f32().unwrap());
  log!("Read u32: {}", reader.read_u32().unwrap());
}
