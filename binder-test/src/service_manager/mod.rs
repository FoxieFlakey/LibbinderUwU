use libbinder::{formats::dead_simple::DeadSimpleFormat, packet::{Packet, builder::PacketBuilder}};
use libbinder_runtime::{Runtime, binder_object::BinderObject};

use crate::common::log;

struct ContextManager;

impl BinderObject for ContextManager {
  fn on_packet(&self, _runtime: &Runtime, packet: &Packet<'_>, reply_builder: &mut PacketBuilder<'_>) {
    log!("Incoming transaction code: {}", packet.get_code());
    reply_builder.set_code(7875);
    
    let mut writer = reply_builder.writer(DeadSimpleFormat::new());
    writer.write_f64(0.872);
    writer.write_f32(0.3);
    writer.write_u32(9);
  }
}

pub fn main() {
  let runtime = Runtime::new().unwrap();
  runtime.become_manager(Box::new(ContextManager)).map_err(|x| x.1).unwrap();
  
  loop { nix::unistd::sleep(1); }
}

