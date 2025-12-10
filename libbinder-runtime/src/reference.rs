use std::{marker::Unsize, ops::Deref, sync::{Arc, LazyLock}};

use libbinder_raw::types::reference::{ObjectRef, ObjectRefRemote};

use crate::{ArcRuntime, binder_object::{self, BinderObject}};

// This calls BC_RELEASE when dropped
pub struct OwnedRemoteRef {
  pub obj_ref: ObjectRefRemote
}

impl Drop for OwnedRemoteRef {
  fn drop(&mut self) {
    if self.obj_ref.data_handle != 0 {
      todo!("Actually perform BC_RELEASE");
    }
  }
}

pub struct Reference<ContextManager: BinderObject<ContextManager>, T: ?Sized> {
  // Concrete callable object
  // which doesn't have to be implementing BinderObject
  pub(crate) concrete: Arc<T>,
  pub(crate) remote_reference: Option<Arc<OwnedRemoteRef>>,
  pub(crate) runtime: ArcRuntime<ContextManager>
}

impl<ContextManager: BinderObject<ContextManager>, T: BinderObject<ContextManager> + ?Sized> Clone for Reference<ContextManager, T> {
  fn clone(&self) -> Self {
    Self {
      concrete: self.concrete.clone(),
      remote_reference: self.remote_reference.clone(),
      runtime: self.runtime.clone()
    }
  }
}

static CTX_MGR_REFERENCE: LazyLock<Arc<OwnedRemoteRef>> = LazyLock::new(|| Arc::new(OwnedRemoteRef { obj_ref: ObjectRefRemote { data_handle: 0 } }));

impl<ContextManager: BinderObject<ContextManager>> Reference<ContextManager, ContextManager> {
  pub(crate) fn context_manager(runtime: ArcRuntime<ContextManager>) -> Self {
    Self {
      concrete: runtime.shared.ctx_manager.read().unwrap().clone().unwrap(),
      remote_reference: Some(CTX_MGR_REFERENCE.clone()),
      runtime: runtime.clone()
    }
  }
}

impl<ContextManager: BinderObject<ContextManager>> Reference<ContextManager, dyn BinderObject<ContextManager>> {
  pub(crate) fn get_concrete(&self) -> &Arc<dyn BinderObject<ContextManager>> {
    &self.concrete
  }
  
  pub(crate) fn get_remote(&self) -> Option<&Arc<OwnedRemoteRef>> {
    self.remote_reference.as_ref()
  }
  
  pub(crate) fn as_raw_object_ref(&self) -> ObjectRef {
    if let Some(remote) = &self.remote_reference {
      ObjectRef::Remote(remote.obj_ref.clone())
    } else {
      ObjectRef::Local(binder_object::into_local_object_ref(&self.concrete))
    }
  }
}

impl<ContextManager: BinderObject<ContextManager>, T> Reference<ContextManager, T> {
  pub fn coerce<U: ?Sized>(self) -> Reference<ContextManager, U>
    where T: Unsize<U>
  {
    Reference {
      concrete: self.concrete as Arc<U>,
      remote_reference: self.remote_reference,
      runtime: self.runtime
    }
  }
}

impl<ContextManager: BinderObject<ContextManager>, T: ?Sized> Deref for Reference<ContextManager, T> {
  type Target = T;
  
  fn deref(&self) -> &Self::Target {
    &self.concrete
  }
}




