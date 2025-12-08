// It is a no brainer object which literally proxies to remote binder
// it is exists for convenient

use std::sync::Arc;

use libbinder::packet::PacketSendError;
use libbinder_raw::types::reference::ObjectRefRemote;

use crate::{Runtime, binder_object::BinderObject};

pub struct ProxyObject<ContextManager: BinderObject<ContextManager>> {
  pub(crate) runtime: Arc<Runtime<ContextManager>>,
  pub(crate) remote_ref: ObjectRefRemote
}

impl<ContextManager: BinderObject<ContextManager>> BinderObject<ContextManager> for ProxyObject<ContextManager> {
  fn on_packet<'runtime>(&self, runtime: &'runtime Arc<Runtime<ContextManager>>, packet: &crate::packet::Packet<'runtime, ContextManager>) -> crate::packet::Packet<'runtime, ContextManager> {
    assert!(Arc::ptr_eq(&self.runtime, runtime), "Attempting to use this binder object on other runtime!");
    match runtime.send_packet(self.remote_ref.clone(), packet) {
      Ok(reply) => reply,
      Err(PacketSendError::DeadTarget) => panic!("Target was dead cannot proxyy over"),
      Err(e) => panic!("Error occur while proxying to remote object: {e:#?}")
    }
  }
}

