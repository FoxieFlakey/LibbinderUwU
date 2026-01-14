use std::borrow::Cow;

use libbinder::{command_buffer::{Command, CommandBuffer}, return_buffer::ReturnBuffer};
use libbinder_raw::types::reference::ObjectRefRemote;

use crate::{WeakRuntime, object::Object, packet::Packet};

pub struct Proxy<Mgr: Object<Mgr>> {
  runtime: WeakRuntime<Mgr>,
  remote_ref: ObjectRefRemote
}

impl<Mgr: Object<Mgr>> Proxy<Mgr> {
  pub(crate) fn new(weak_rt: WeakRuntime<Mgr>, remote_ref: ObjectRefRemote) -> Self {
    Self {
      runtime: weak_rt,
      remote_ref
    }
  }
}

impl<Mgr: Object<Mgr>> Object<Mgr> for Proxy<Mgr> {
  fn do_transaction<'runtime>(&self, packet: &Packet<'_, Mgr>) -> Result<Packet<'runtime, Mgr>, Packet<'runtime, Mgr>> {
    assert!(
      self.runtime.ptr_eq(&packet.get_runtime().downgrade()),
      "attempting to send packet belonging to other runtime"
    );
    
    let binder_dev = packet.get_runtime().get_binder();
    let mut ret_buf = ReturnBuffer::new(binder_dev, 64 * 1024);
    let mut cmd_buf = CommandBuffer::new(binder_dev);
    
    cmd_buf.enqueue_command(Command::SendTransaction(self.remote_ref, Cow::Borrowed(&packet.packet)));
    match cmd_buf.exec_always_block(Some(&mut ret_buf)) {
      Ok(()) => {},
      Err((idx, e)) => {
        panic!("Error executing command at index {idx} because of: {e}")
      }
    }
    
    todo!();
  }
}

pub struct SelfMananger(pub Proxy<SelfMananger>);

impl Object<SelfMananger> for SelfMananger {
  fn do_transaction<'runtime>(&self, packet: &Packet<'_, SelfMananger>) -> Result<Packet<'runtime, SelfMananger>, Packet<'runtime, SelfMananger>> {
    self.0.do_transaction(packet)
  }
}

