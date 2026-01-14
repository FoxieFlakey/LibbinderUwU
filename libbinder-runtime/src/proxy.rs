use std::{borrow::Cow, error::Error, fmt::Display};

use libbinder::{command_buffer::{Command, CommandBuffer}, return_buffer::{ReturnBuffer, ReturnValue}};
use libbinder_raw::types::reference::ObjectRefRemote;

use crate::{WeakRuntime, object::{Object, TransactionError}, packet::Packet};

pub struct Proxy<Mgr: Object<Mgr>> {
  runtime: WeakRuntime<Mgr>,
  remote_ref: ObjectRefRemote
}

impl<Mgr: Object<Mgr>> Proxy<Mgr> {
  pub(crate) fn new(weak_rt: WeakRuntime<Mgr>, remote_ref: ObjectRefRemote) -> Self {
    Self {
      runtime: weak_rt,
      remote_ref
    }
  }
}

impl<Mgr: Object<Mgr>> Object<Mgr> for Proxy<Mgr> {
  fn do_transaction<'runtime>(&self, packet: &'runtime Packet<'runtime, Mgr>) -> Result<Packet<'runtime, Mgr>, TransactionError> {
    assert!(
      self.runtime.ptr_eq(&packet.get_runtime().downgrade()),
      "attempting to send packet belonging to other runtime"
    );
    
    let rt: &'runtime crate::ArcRuntime<Mgr> = packet.get_runtime();
    let binder_dev: std::os::unix::prelude::BorrowedFd<'runtime> = rt.get_binder();
    let mut ret_buf: ReturnBuffer<'runtime> = ReturnBuffer::new(binder_dev, 64 * 1024);
    let mut cmd_buf = CommandBuffer::new(binder_dev);
    
    cmd_buf.enqueue_command(Command::SendTransaction(self.remote_ref, Cow::Borrowed(&packet.packet)));
    match cmd_buf.exec_always_block(Some(&mut ret_buf)) {
      Ok(()) => {},
      Err((idx, e)) => {
        panic!("Error executing command at index {idx} because of: {e}")
      }
    }
    
    for ret in ret_buf.get_parsed() {
      match ret {
        ReturnValue::Transaction(_) => todo!(),
        ReturnValue::Acquire(object_ref_local) => todo!(),
        ReturnValue::AcquireWeak(object_ref_local) => todo!(),
        ReturnValue::Release(object_ref_local) => todo!(),
        ReturnValue::ReleaseWeak(object_ref_local) => todo!(),
        ReturnValue::Reply(packet) => {
          return Ok(Packet {
            runtime: rt,
            packet: packet.clone()
          })
        },
        ReturnValue::TransactionFailed => todo!(),
        ReturnValue::Ok => todo!(),
        ReturnValue::Error(_) => todo!(),
        ReturnValue::SpawnLooper => todo!(),
        ReturnValue::TransactionComplete => todo!(),
        ReturnValue::DeadReply => return Err(TransactionError::DeadTarget),
        ReturnValue::Noop => ()
      }
    }
    
    #[derive(Debug)]
    struct ErrorMsg(&'static str);
    
    impl Display for ErrorMsg {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
      }
    }
    
    impl Error for ErrorMsg {
    }
    
    Err(TransactionError::MiscellanousError(Box::new(ErrorMsg("Error Unexpected"))))
  }
}

pub struct SelfMananger(pub Proxy<SelfMananger>);

impl Object<SelfMananger> for SelfMananger {
  fn do_transaction<'runtime>(&self, packet: &'runtime Packet<'runtime, SelfMananger>) -> Result<Packet<'runtime, SelfMananger>, TransactionError> {
    self.0.do_transaction(packet)
  }
}

