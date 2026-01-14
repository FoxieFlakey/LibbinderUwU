use std::{borrow::Cow, os::fd::AsFd};

use libbinder::{command_buffer::{Command, CommandBuffer}, packet::Packet};
use libbinder_raw::types::reference::ObjectRefRemote;

use crate::{WeakRuntime, object::Object};

pub struct RemoteObject<Mgr: Object<Mgr>> {
  runtime: WeakRuntime<Mgr>,
  remote_ref: ObjectRefRemote
}

impl<Mgr: Object<Mgr>> Object<Mgr> for RemoteObject<Mgr> {
  fn on_packet<'runtime>(&self, packet: &Packet<'runtime>) -> Result<Packet<'runtime>, Packet<'runtime>> {
    let runtime = self.runtime.clone().upgrade().unwrap();
    let binder_dev = runtime.inner.binder_dev.as_fd();
    
    let mut cmd_buf = CommandBuffer::new(binder_dev);
    cmd_buf.enqueue_command(Command::SendTransaction(Cow::Borrowed(packet)));
    runtime.run_commands(&mut cmd_buf, |v| {
      
    }).unwrap();
    
    todo!();
  }
}


