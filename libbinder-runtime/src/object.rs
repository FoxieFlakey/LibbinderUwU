use std::any::Any;
use crate::packet::Packet;

// About storing ArcRuntime, caller should store only weak
// reference to the runtime, don't store strong reference
//
// Runtime will store the strong reference to object if its
// sent outside
pub trait Object<Mgr: Object<Mgr>>: Sync + Send + Any + 'static {
  // Both Ok and Err, are sent anyway, they're provided leik this
  // instead singular infallible just to give QoL improvement for the
  // implementer to be able to use Rust's ? and plus the runtime
  // doesn't really care if its Ok or Err, at the end it gets a
  // packet lol
  fn do_transaction<'runtime>(&self, packet: &Packet<'_, Mgr>) -> Result<Packet<'runtime, Mgr>, Packet<'runtime, Mgr>>;
}



