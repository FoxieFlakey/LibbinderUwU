// It is a no brainer object which literally proxies to remote binder
// it is exists for convenient

use std::sync::Arc;

use libbinder::packet::PacketSendError;
use libbinder_raw::types::reference::ObjectRefRemote;

use crate::{Runtime, binder_object::{BinderObject, ConreteObjectFromRemote}, packet::Packet};

pub struct ProxyObject<ContextManager: BinderObject<ContextManager>> {
  pub(crate) runtime: Arc<Runtime<ContextManager>>,
  pub(crate) remote_ref: ObjectRefRemote
}

impl<ContextManager: BinderObject<ContextManager>> BinderObject<ContextManager> for ProxyObject<ContextManager> {
  fn on_packet<'runtime>(&self, runtime: &'runtime Arc<Runtime<ContextManager>>, packet: &Packet<'runtime, ContextManager>) -> crate::packet::Packet<'runtime, ContextManager> {
    assert!(Arc::ptr_eq(&self.runtime, runtime), "Attempting to use this binder object on other runtime!");
    match runtime.send_packet(self.remote_ref.clone(), packet) {
      Ok(reply) => reply,
      Err(PacketSendError::DeadTarget) => panic!("Target was dead cannot proxyy over"),
      Err(e) => panic!("Error occur while proxying to remote object: {e:#?}")
    }
  }
}

pub struct GenericContextManager {
  proxy: ProxyObject<GenericContextManager>
}

impl BinderObject<GenericContextManager> for GenericContextManager {
  fn on_packet<'runtime>(&self, runtime: &'runtime Arc<Runtime<GenericContextManager>>, packet: &Packet<'runtime, GenericContextManager>) -> Packet<'runtime, GenericContextManager> {
    self.proxy.on_packet(runtime, packet)
  }
}

impl ConreteObjectFromRemote<GenericContextManager> for GenericContextManager {
  fn try_from_remote(_runtime: &Arc<Runtime<Self>>, proxy: ProxyObject<Self>) -> Result<Self, ()> {
    Ok(Self {
      proxy
    }) 
  }
}


