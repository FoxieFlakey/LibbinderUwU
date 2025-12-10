// It is a no brainer object which literally proxies to remote binder
// it is exists for convenient

use std::sync::Arc;

use either::Either;
use libbinder::packet::PacketSendError;

use crate::{ArcRuntime, binder_object::{BinderObject, CreateInterfaceObject}, packet::Packet, reference::{OwnedRemoteRef, Reference}};

pub struct Object<ContextManager: BinderObject<ContextManager>> {
  pub(crate) runtime: ArcRuntime<ContextManager>,
  pub(crate) reference: Either<Arc<OwnedRemoteRef>, Reference<ContextManager, dyn BinderObject<ContextManager>>>
}

impl<ContextManager: BinderObject<ContextManager>> BinderObject<ContextManager> for Object<ContextManager> {
  fn on_packet<'runtime>(&self, runtime: &'runtime ArcRuntime<ContextManager>, packet: &Packet<'runtime, ContextManager>) -> crate::packet::Packet<'runtime, ContextManager> {
    assert!(Arc::ptr_eq(&self.runtime.inner, &runtime.inner), "Attempting to use this binder object on other runtime!");
    match &self.reference {
      Either::Left(remote) => {
        match runtime.send_packet(remote.obj_ref.clone(), packet) {
          Ok(reply) => reply,
          Err(PacketSendError::DeadTarget) => panic!("Target was dead cannot proxyy over"),
          Err(e) => panic!("Error occur while proxying to remote object: {e:#?}")
        }
      }
      
      Either::Right(local) => {
        local.on_packet(runtime, packet)
      }
    }
  }
}

impl<ContextManager: BinderObject<ContextManager>> CreateInterfaceObject<ContextManager> for Object<ContextManager> {
  fn try_from_remote(runtime: &ArcRuntime<ContextManager>, remote_ref: Object<ContextManager>) -> Result<Self, ()> {
    assert!(ArcRuntime::ptr_eq(&runtime, &remote_ref.runtime));
    Ok(remote_ref)
  }
}

pub struct GenericContextManager {
  proxy: Object<Self>
}

impl BinderObject<Self> for GenericContextManager {
  fn on_packet<'runtime>(&self, runtime: &'runtime ArcRuntime<Self>, packet: &Packet<'runtime, Self>) -> Packet<'runtime, Self> {
    self.proxy.on_packet(runtime, packet)
  }
}

impl CreateInterfaceObject<Self> for GenericContextManager {
  fn try_from_remote(_runtime: &ArcRuntime<Self>, proxy: Object<Self>) -> Result<Self, ()> {
    Ok(Self {
      proxy
    }) 
  }
}


