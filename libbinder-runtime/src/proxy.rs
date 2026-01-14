use std::{borrow::Cow, error::Error, fmt::Display};

use libbinder::{command_buffer::Command, return_buffer::ReturnValue};
use libbinder_raw::types::reference::ObjectRefRemote;

use crate::{WeakRuntime, context::Context, object::{Object, TransactionError}, packet::Packet};

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
  fn do_transaction<'packet, 'runtime>(&self, packet: &'packet Packet<'runtime, Mgr>) -> Result<Packet<'runtime, Mgr>, TransactionError> {
    assert!(
      self.runtime.ptr_eq(&packet.get_runtime().downgrade()),
      "attempting to send packet belonging to other runtime"
    );
    
    let rt = packet.get_runtime();
    let ctx = rt.____rt.exec_context.get_or(|| Context::new(rt.clone()));
    let mut ret = None;
    
    ctx.exec(rt, |cmd_buf| {
      cmd_buf.enqueue_command(Command::SendTransaction(self.remote_ref.clone(), Cow::Borrowed(&packet.packet)));
    }, |v| {
      match v {
        ReturnValue::Transaction(_) => todo!(),
        ReturnValue::Acquire(_) => todo!(),
        ReturnValue::AcquireWeak(_) => todo!(),
        ReturnValue::Release(_) => todo!(),
        ReturnValue::ReleaseWeak(_) => todo!(),
        ReturnValue::Reply(packet) => {
          assert!(ret.is_none());
          ret = Some(Ok(Packet {
            runtime: rt,
            packet: packet.clone()
          }));
        },
        ReturnValue::TransactionFailed => todo!(),
        ReturnValue::Ok => todo!(),
        ReturnValue::Error(_) => todo!(),
        ReturnValue::SpawnLooper => todo!(),
        ReturnValue::TransactionComplete => println!("Transaction complete"),
        ReturnValue::DeadReply => {
          assert!(ret.is_none());
          ret = Some(Err(TransactionError::DeadTarget));
        },
        ReturnValue::Noop => ()
      }
    });
    
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
  fn do_transaction<'packet, 'runtime>(&self, packet: &'packet Packet<'runtime, SelfMananger>) -> Result<Packet<'runtime, SelfMananger>, TransactionError> {
    self.0.do_transaction(packet)
  }
}

