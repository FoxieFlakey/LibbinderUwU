use std::sync::Arc;

use libbinder_raw::types::reference::ObjectRefRemote;

use crate::{ArcRuntime, boxed_object::BoxedObject, object::Object};

pub struct LocalObject<Mgr: Object<Mgr>, T: Object<Mgr>> {
  pub(crate) runtime: ArcRuntime<Mgr>,
  pub(crate) inner: BoxedObject<Mgr>,
  typed: Arc<T>
}

pub struct RemoteObject<Mgr: Object<Mgr>, T: Object<Mgr>> {
  pub(crate) runtime: ArcRuntime<Mgr>,
  pub(crate) inner: ObjectRefRemote,
  typed: Arc<T>
}

pub enum Reference<Mgr: Object<Mgr>, T: Object<Mgr>> {
  Local(Arc<LocalObject<Mgr, T>>),
  Remote(Arc<RemoteObject<Mgr, T>>)
}

impl<Mgr: Object<Mgr>, T: Object<Mgr>> Clone for Reference<Mgr, T> {
  fn clone(&self) -> Self {
    match self {
      Reference::Local(x) => Self::Local(x.clone()),
      Reference::Remote(x) => Self::Remote(x.clone())
    }
  }
}

impl<Mgr: Object<Mgr>, T: Object<Mgr>> Reference<Mgr, T> {
  pub fn get(&self) -> &Arc<T> {
    match self {
      Reference::Local(x) => &x.typed,
      Reference::Remote(x) => &x.typed
    }
  }
}

