use std::{cell::{RefCell, RefMut}, collections::VecDeque, io, mem, ops::DerefMut, os::fd::{AsFd, OwnedFd}, sync::Arc};

use libbinder::{command_buffer::CommandBuffer, packet::Packet, return_buffer::{ReturnBuffer, ReturnValue}};
use libbinder_raw::types::reference::ObjectRefLocal;

use crate::{ArcRuntime, WeakRuntime, object::Object};

pub struct ExecContext<Mgr: Object<Mgr>> {
  runtime: WeakRuntime<Mgr>,
  return_buffer: RefCell<ReturnBuffer<'static>>,
  binder_dev: Arc<OwnedFd>
}

impl<Mgr: Object<Mgr>> ExecContext<Mgr> {
  fn handle_ret_val<'ret_buf: 'rt, 'rt>(&self, queue: &mut VecDeque<(&'ret_buf ObjectRefLocal, &'ret_buf Packet<'rt>)>, val: &'ret_buf ReturnValue<'rt>) -> bool {
    match val {
      ReturnValue::Ok => false,
      ReturnValue::Noop => true,
      ReturnValue::Error(x) => {
        panic!("Calling to binder returned error: {}", nix::Error::from_raw(*x).desc());
      },
      ReturnValue::SpawnLooper => true,
      ReturnValue::TransactionComplete => false,
      ReturnValue::DeadReply => false,
      ReturnValue::Reply(_) => false,
      ReturnValue::Release(_) => true,
      ReturnValue::Acquire(_) => true,
      ReturnValue::ReleaseWeak(_) => true,
      ReturnValue::AcquireWeak(_) => true,
      // Lets handle this later
      ReturnValue::Transaction((obj_ref, packet)) => {
        queue.push_back((obj_ref, packet));
        true
      },
      ReturnValue::TransactionFailed => false
    }
  }
  
  // 'handle_retval' only called if one of the return values in 'handle_ret_val' is false
  pub fn run_commands<F: FnMut(&ReturnValue)>(&self, commands: &mut CommandBuffer, mut handle_retval: F) -> Result<(), (usize, io::Error)> {
    let mut ret_buf_borrow = self.return_buffer.borrow_mut();
    let ret_buf = unsafe { mem::transmute::<&mut ReturnBuffer<'static>, &mut ReturnBuffer<'_>>(RefMut::deref_mut(&mut ret_buf_borrow)) };
    commands.exec_always_block(Some(ret_buf))?;
    
    // Handle the return values
    let mut queued_for_later = VecDeque::new();
    for val in ret_buf.get_parsed().iter() {
      if self.handle_ret_val(&mut queued_for_later, &val) {
        continue;
      }
      
      handle_retval(&val);
    }
    ret_buf.clear();
    drop(ret_buf_borrow);
    
    Ok(())
  }
  
  pub fn new(runtime: &ArcRuntime<Mgr>) -> Self {
    Self {
      return_buffer: RefCell::new(unsafe { mem::transmute::<ReturnBuffer<'_>, ReturnBuffer<'static>>(ReturnBuffer::new(runtime.inner.binder_dev.as_fd(), 4 * 1024 * 1024)) }),
      binder_dev: runtime.inner.binder_dev.clone(),
      runtime: runtime.downgrade()
    }
  }
}


