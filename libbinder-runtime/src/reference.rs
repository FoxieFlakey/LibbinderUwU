use std::sync::Arc;

use libbinder_raw::types::reference::{ObjectRef, ObjectRefLocal, ObjectRefRemote};

use crate::{ArcRuntime, object::{self, Object}};

pub struct LocalObject<Mgr: Object<Mgr> + ?Sized, T: Object<Mgr> + ?Sized> {
  pub(crate) runtime: ArcRuntime<Mgr>,
  pub(crate) inner: ObjectRefLocal,
  pub(crate) typed: Arc<T>
}

pub struct RemoteObject<Mgr: Object<Mgr> + ?Sized, T: Object<Mgr> + ?Sized> {
  pub(crate) runtime: ArcRuntime<Mgr>,
  pub(crate) inner: ObjectRefRemote,
  pub(crate) typed: Arc<T>
}

pub enum Reference<Mgr: Object<Mgr> + ?Sized, T: Object<Mgr> + ?Sized> {
  Local(Arc<LocalObject<Mgr, T>>),
  Remote(Arc<RemoteObject<Mgr, T>>)
}

impl<Mgr: Object<Mgr> + ?Sized, T: Object<Mgr> + ?Sized> Clone for Reference<Mgr, T> {
  fn clone(&self) -> Self {
    match self {
      Reference::Local(x) => Self::Local(x.clone()),
      Reference::Remote(x) => Self::Remote(x.clone())
    }
  }
}

impl<Mgr: Object<Mgr> + ?Sized, T: Object<Mgr>> Reference<Mgr, T> {
  pub fn from_local(runtime: ArcRuntime<Mgr>, local: Arc<T>) -> Self {
    Self::Local(Arc::new(LocalObject {
      inner: object::into_local_ref(local.clone()),
      typed: local,
      runtime
    }))
  }
}

impl<Mgr: Object<Mgr> + ?Sized, T: Object<Mgr> + ?Sized> Reference<Mgr, T> {
  pub fn get(&self) -> &Arc<T> {
    match self {
      Reference::Local(x) => &x.typed,
      Reference::Remote(x) => &x.typed
    }
  }
  
  pub(crate) fn get_runtime(this: &Self) -> &ArcRuntime<Mgr> {
    match this {
      Reference::Local(x) => &x.runtime,
      Reference::Remote(x) => &x.runtime
    }
  }
  
  pub(crate) fn get_obj_ref(this: &Self) -> ObjectRef {
    match this {
      Reference::Local(x) => {
        ObjectRef::Local(x.inner.clone())
      },
      Reference::Remote(x) => {
        ObjectRef::Remote(x.inner.clone())
      }
    }
  }
}

