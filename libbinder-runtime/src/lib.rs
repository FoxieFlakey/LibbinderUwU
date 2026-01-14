use std::{cell::RefCell, io, marker::PhantomData, os::fd::OwnedFd, sync::{Arc, Weak}};

use libbinder::{command_buffer::CommandBuffer, return_buffer::ReturnValue};
use thread_local::ThreadLocal;

use crate::{execution_context::ExecContext, object::Object, util::pool::Pool};

pub mod object;
pub mod remote_object;
pub mod packet;
mod util;
mod execution_context;

pub struct ArcRuntime<Mgr: Object<Mgr>> {
  inner: Arc<Runtime<Mgr>>
}

pub struct WeakRuntime<Mgr: Object<Mgr>> {
  inner: Weak<Runtime<Mgr>>
}

impl<Mgr: Object<Mgr>> Clone for WeakRuntime<Mgr> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone()
    }
  }
}

impl<Mgr: Object<Mgr>> Clone for ArcRuntime<Mgr> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone()
     }
  }
}

pub struct Runtime<Mgr: Object<Mgr>> {
  binder_dev: Arc<OwnedFd>,
  exec_contexts: Pool<ExecContext<Mgr>>,
  local_contexts: ThreadLocal<RefCell<ExecContext<Mgr>>>,
  _phantom: PhantomData<Mgr>
}

impl<Mgr: Object<Mgr>> ArcRuntime<Mgr> {
  pub fn downgrade(&self) -> WeakRuntime<Mgr> {
    WeakRuntime {
      inner: Arc::downgrade(&self.inner)
    }
  }
  
  pub fn run_commands<F: FnMut(&ReturnValue)>(&self, commands: &mut CommandBuffer, handle_retval: F) -> Result<(), (usize, io::Error)> {
    let ctx = self.inner.local_contexts.get_or(|| RefCell::new(self.inner.exec_contexts.get().into())).borrow_mut();
    ctx.run_commands(commands, handle_retval)
  }
}

impl<Mgr: Object<Mgr>> WeakRuntime<Mgr> {
  pub fn upgrade(self) -> Option<ArcRuntime<Mgr>> {
    Some(ArcRuntime {
      inner: self.inner.upgrade()?
    })
  }
}

