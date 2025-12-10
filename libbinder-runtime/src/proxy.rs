// It is a no brainer object which literally proxies to remote binder
// it is exists for convenient

use std::sync::Arc;

use libbinder::packet::PacketSendError;

use crate::{Runtime, binder_object::{BinderObject, CreateInterfaceObject}, packet::Packet, reference::OwnedRemoteRef};

pub struct ProxyObject<ContextManager: BinderObject<ContextManager>> {
  pub(crate) runtime: Arc<Runtime<ContextManager>>,
  pub(crate) remote_ref: Arc<OwnedRemoteRef>
}

impl<ContextManager: BinderObject<ContextManager>> BinderObject<ContextManager> for ProxyObject<ContextManager> {
  fn on_packet<'runtime>(&self, runtime: &'runtime Arc<Runtime<ContextManager>>, packet: &Packet<'runtime, ContextManager>) -> crate::packet::Packet<'runtime, ContextManager> {
    assert!(Arc::ptr_eq(&self.runtime, runtime), "Attempting to use this binder object on other runtime!");
    match Runtime::send_packet(runtime, self.remote_ref.obj_ref.clone(), packet) {
      Ok(reply) => reply,
      Err(PacketSendError::DeadTarget) => panic!("Target was dead cannot proxyy over"),
      Err(e) => panic!("Error occur while proxying to remote object: {e:#?}")
    }
  }
}

pub struct GenericContextManager {
  proxy: ProxyObject<Self>
}

impl BinderObject<Self> for GenericContextManager {
  fn on_packet<'runtime>(&self, runtime: &'runtime Arc<Runtime<Self>>, packet: &Packet<'runtime, Self>) -> Packet<'runtime, Self> {
    self.proxy.on_packet(runtime, packet)
  }
}

impl CreateInterfaceObject<Self> for GenericContextManager {
  fn try_from_remote(_runtime: &Arc<Runtime<Self>>, proxy: ProxyObject<Self>) -> Result<Self, ()> {
    Ok(Self {
      proxy
    }) 
  }
}


