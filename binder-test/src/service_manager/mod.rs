use std::sync::Arc;

use libbinder::{formats::dead_simple::DeadSimpleFormat, packet::{Packet, builder::PacketBuilder}};
use libbinder_runtime::{Runtime, binder_object::BinderObject};

use crate::common::log;

struct ContextManager;

impl ContextManager {
  pub fn bwah_uwu(&self) {
    println!("hi uwu");
  }
}

impl BinderObject<ContextManager> for ContextManager {
  fn on_packet(&self, _runtime: &Arc<Runtime<ContextManager>>, packet: &Packet<'_>, reply_builder: &mut PacketBuilder) {
    log!("Incoming transaction code: {}", packet.get_code());
    reply_builder.set_code(7875);
    
    let mut writer = reply_builder.writer(DeadSimpleFormat::new());
    writer.write_f64(0.872);
    writer.write_f32(0.3);
    writer.write_u32(9);
  }
}

pub fn main() {
  let runtime = Runtime::new_as_manager(Arc::new(ContextManager))
    .ok()
    .unwrap();
  
  let ctx = runtime.get_context_manager(); 
  ctx.bwah_uwu();
  loop { nix::unistd::sleep(1); }
}

