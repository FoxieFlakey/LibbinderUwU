use std::borrow::Cow;

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
    let ctx = rt.____rt.exec_context.get_or(|| Context::new(rt.get_binder()));
    let mut ret = None;
    
    let mut has_transaction_complete = false;
    let mut has_failed = false;
    
    // Send the reply
    ctx.exec(rt, |cmd_buf| {
      cmd_buf.enqueue_command(Command::SendTransaction(self.remote_ref.clone(), Cow::Borrowed(&packet.packet)));
    }, |v| {
      match v {
        ReturnValue::Transaction(_) => todo!(),
        ReturnValue::Acquire(_) => todo!(),
        ReturnValue::AcquireWeak(_) => todo!(),
        ReturnValue::Release(_) => todo!(),
        ReturnValue::ReleaseWeak(_) => todo!(),
        ReturnValue::Reply(_) => {
          panic!("Reply is not expected here");
        },
        ReturnValue::TransactionFailed => has_failed = true,
        ReturnValue::Ok => todo!(),
        ReturnValue::Error(_) => todo!(),
        ReturnValue::SpawnLooper => todo!(),
        ReturnValue::TransactionComplete => {
          println!("Transaction complete");
          has_transaction_complete = true;
        },
        ReturnValue::DeadReply => {
          assert!(ret.is_none());
          ret = Some(Err(TransactionError::UnreachableTarget));
        },
        ReturnValue::Noop => ()
      }
    });
    
    if !has_transaction_complete {
      panic!("Kernel unable to send transaction!");
    }
    
    // Then read the result
    ctx.exec(rt, |_| (), |v| {
      match v {
        ReturnValue::Reply(packet) => {
          assert!(ret.is_none());
          ret = Some(Ok(Packet {
            runtime: rt,
            packet: packet.clone()
          }));
        },
        ReturnValue::Noop => (),
        _ => panic!("Unexpected")
      }
    });
    
    if let Some(x) = ret {
      x
    } else {
      match (has_transaction_complete, has_failed) {
        (true, false) => panic!("kernel didnt reply back"),
        (true, true) => Err(TransactionError::FailedReply),
        (false, _) => panic!("kernel did not response")
      }
    }
  }
}

pub struct SelfMananger(pub Proxy<SelfMananger>);

impl Object<SelfMananger> for SelfMananger {
  fn do_transaction<'packet, 'runtime>(&self, packet: &'packet Packet<'runtime, SelfMananger>) -> Result<Packet<'runtime, SelfMananger>, TransactionError> {
    self.0.do_transaction(packet)
  }
}

