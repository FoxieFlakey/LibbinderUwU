use libbinder::{formats::dead_simple::DeadSimpleFormatReader, packet::builder::PacketBuilder};
use libbinder_raw::CONTEXT_MANAGER_REF;
use libbinder_runtime::{Runtime, binder_object::GenericContextManager};

use crate::common::log;

pub fn main() {
  let runtime = Runtime::<GenericContextManager>::new().ok().unwrap();
  nix::unistd::sleep(1);
  
  let response = runtime.send_packet(
      CONTEXT_MANAGER_REF,
      &PacketBuilder::new()
        .set_code(80386)
        .build(runtime.get_binder())
    ).unwrap();
  
  assert!(response.get_code() == 7875);
  
  let mut reader = response.reader(DeadSimpleFormatReader::new());
  log!("Read f64: {}", reader.read_f64().unwrap());
  log!("Read f32: {}", reader.read_f32().unwrap());
  log!("Read u32: {}", reader.read_u32().unwrap());
}
