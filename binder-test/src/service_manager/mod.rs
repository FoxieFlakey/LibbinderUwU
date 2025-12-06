use std::sync::Arc;

use libbinder::{formats::dead_simple::DeadSimpleFormat, packet::{Packet, builder::PacketBuilder}};
use libbinder_runtime::{Runtime, binder_object::BinderObject};

use crate::{common::log, interface::IServiceManager};

struct ContextManager;

impl IServiceManager for ContextManager {
  fn length_of_string(&self, string: &str) -> usize {
    log!("Length of string is {}", string.len());
    string.len()
  }
  
  fn bwah_uwu(&self, data: &str) {
    log!("Bwah_uwu: {}", data);
  }
}

impl BinderObject<ContextManager> for ContextManager {
  fn on_packet(&self, _runtime: &Arc<Runtime<ContextManager>>, packet: &Packet<'_>, reply_builder: &mut PacketBuilder) {
    match packet.get_code() {
      _ => {
        log!("Received unknown transaction code: {}", packet.get_code());
        reply_builder.set_code(0x01);
        reply_builder.writer(DeadSimpleFormat::new())
          .write_str("unknown transaction code");
      }
    }
  }
}

pub fn main() {
  let runtime = Runtime::new_as_manager(Arc::new(ContextManager))
    .ok()
    .unwrap();
  
  let ctx = runtime.get_context_manager(); 
  ctx.bwah_uwu("HI UwU");
  loop { nix::unistd::sleep(1); }
}

