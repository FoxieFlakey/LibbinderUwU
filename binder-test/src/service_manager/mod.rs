use libbinder::packet::{Packet, builder::PacketBuilder};
use libbinder_runtime::{Runtime, binder_object::BinderObject};

struct ContextManager;

impl BinderObject for ContextManager {
  fn on_packet(&self, runtime: &Runtime, packet: &Packet<'_>, reply_builder: &mut PacketBuilder<'_>) {
    println!("Incoming transaction code: {}", packet.get_code());
    reply_builder.set_code(7875);
  }
}

pub fn main() {
  let runtime = Runtime::new().unwrap();
  runtime.become_manager(Box::new(ContextManager)).map_err(|x| x.1).unwrap();
  
  loop { nix::unistd::sleep(1); }
}

