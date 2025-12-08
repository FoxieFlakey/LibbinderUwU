use std::{marker::{PhantomData, Unsize}, ops::{CoerceUnsized, Deref}, sync::{Arc, LazyLock}};

use libbinder_raw::types::reference::{ObjectRef, ObjectRefRemote};

use crate::{Runtime, binder_object::{self, BinderObject}};

// This calls BC_RELEASE when dropped
struct OwnedRemoteRef {
  obj_ref: ObjectRefRemote
}

impl Drop for OwnedRemoteRef {
  fn drop(&mut self) {
    if self.obj_ref.data_handle != 0 {
      todo!("Actually perform BC_RELEASE");
    }
  }
}

pub struct Reference<'runtime, ContextManager: BinderObject<ContextManager>, T: ?Sized> {
  // Concrete callable object
  // which doesn't have to be implementing BinderObject
  concrete: Arc<T>,
  remote_reference: Option<Arc<OwnedRemoteRef>>,
  phantom: PhantomData<&'runtime Runtime<ContextManager>>
}

impl<'runtime, ContextManager: BinderObject<ContextManager>, T: BinderObject<ContextManager> + ?Sized> Clone for Reference<'runtime, ContextManager, T> {
  fn clone(&self) -> Self {
    Self {
      concrete: self.concrete.clone(),
      remote_reference: self.remote_reference.clone(),
      phantom: PhantomData {}
    }
  }
}

static CTX_MGR_REFERENCE: LazyLock<Arc<OwnedRemoteRef>> = LazyLock::new(|| Arc::new(OwnedRemoteRef { obj_ref: ObjectRefRemote { data_handle: 0 } }));

impl<'runtime, ContextManager: BinderObject<ContextManager>> Reference<'runtime, ContextManager, ContextManager> {
  pub(crate) fn context_manager(runtime: &'runtime Runtime<ContextManager>) -> Self {
    Self {
      concrete: runtime.shared.ctx_manager.read().unwrap().clone().unwrap(),
      remote_reference: Some(CTX_MGR_REFERENCE.clone()),
      phantom: PhantomData {}
    }
  }
}

impl<'runtime, ContextManager: BinderObject<ContextManager>> Reference<'runtime, ContextManager, dyn BinderObject<ContextManager>> {
  pub(crate) fn as_raw_object_ref(&self) -> ObjectRef {
    if let Some(remote) = &self.remote_reference {
      ObjectRef::Remote(remote.obj_ref.clone())
    } else {
      ObjectRef::Local(binder_object::into_local_object_ref(&self.concrete))
    }
  }
}

impl<ContextManager: BinderObject<ContextManager>, T: ?Sized> Deref for Reference<'_, ContextManager, T> {
  type Target = T;
  
  fn deref(&self) -> &Self::Target {
    &self.concrete
  }
}

impl<ContextManager: BinderObject<ContextManager>, T: ?Sized + Unsize<U> + BinderObject<ContextManager>, U: ?Sized> CoerceUnsized<Reference<'_, ContextManager, U>> for Reference<'_, ContextManager, T> {}



