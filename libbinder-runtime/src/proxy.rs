use std::{borrow::Cow, mem::ManuallyDrop, sync::{Arc, atomic::{AtomicU64, Ordering}}};

use libbinder::{command_buffer::{Command, CommandBuffer}, return_buffer::ReturnValue};
use libbinder_raw::types::reference::{CONTEXT_MANAGER_REF, ObjectRef, ObjectRefRemote};

use crate::{ArcRuntime, WeakRuntime, context::Context, object::{self, FromProxy, Object, TransactionError}, packet::Packet};

pub struct Proxy<Mgr: Object<Mgr>> {
  runtime: WeakRuntime<Mgr>,
  remote_ref: ObjectRefRemote
}

impl<Mgr: Object<Mgr>> Drop for Proxy<Mgr> {
  fn drop(&mut self) {
    if self.remote_ref == CONTEXT_MANAGER_REF {
      // Context manager does not need BC_RELEASE or anything
      return;
    }
    
    if let Some(rt) = self.runtime.upgrade() {
      let count_before = rt.____rt.remote_reference_counters.read().unwrap().get(&self.remote_ref).unwrap().fetch_sub(1, Ordering::Relaxed);
      if count_before > 1 {
        // There other reference do nothing
        return;
      }
      
      let mut cmd_buf = CommandBuffer::new(rt.get_binder());
      cmd_buf.enqueue_command(Command::Release(self.remote_ref.clone()));
      cmd_buf.exec_always_block(None).unwrap();
      
      rt.____rt.remote_reference_counters.write()
        .unwrap()
        .remove(&self.remote_ref);
    }
  }
}

impl<Mgr: Object<Mgr>> Proxy<Mgr> {
  pub(crate) fn new(weak_rt: WeakRuntime<Mgr>, remote_ref: ObjectRefRemote) -> Self {
    Self {
      runtime: weak_rt,
      remote_ref
    }
  }
  
  pub fn get_runtime(&self) -> ArcRuntime<Mgr> {
    self.runtime.upgrade().unwrap()
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
    
    for (_, reference) in packet.iter_references() {
      match reference {
        ObjectRef::Local(local) => {
          let obj = ManuallyDrop::new(unsafe { object::from_local_ref::<Mgr>(local) });
          unsafe { Arc::increment_strong_count(Arc::as_ptr(&obj)) };
          
          rt.____rt.reference_states.lock()
            .unwrap()
            .entry(local)
            .or_insert((true, false))
            .0 = true;
        },
        ObjectRef::Remote(remote) => {
          // Get counter for the remote reference
          let read_guard = rt.____rt.remote_reference_counters.read().unwrap() ;
          if let Some(counter) = read_guard.get(&remote) {
            counter.fetch_add(1, Ordering::Relaxed);
          } else {
            drop(read_guard);
            
            rt.____rt.remote_reference_counters.write()
              .unwrap()
              .entry(remote)
              .or_insert(AtomicU64::new(0))
              .fetch_add(1, Ordering::Relaxed);
          }
        }
      }
    }
    
    // Send the reply
    ctx.exec_without_ret(rt, |cmd_buf| {
      cmd_buf.enqueue_command(Command::SendTransaction(self.remote_ref.clone(), Cow::Borrowed(&packet.packet)));
    });
    
    // Then read the result
    ctx.exec(rt, |_| (), |v| {
      match v {
        ReturnValue::Noop => (),
        ReturnValue::Transaction(_) => todo!(),
        ReturnValue::Acquire(_) => todo!(),
        ReturnValue::AcquireWeak(_) => todo!(),
        ReturnValue::Release(_) => todo!(),
        ReturnValue::ReleaseWeak(_) => todo!(),
        ReturnValue::Reply(packet) => {
          assert!(ret.is_none());
          ret = Some(Ok(Packet::new(rt, packet.clone())));
        },
        ReturnValue::TransactionFailed => has_failed = true,
        ReturnValue::Ok => todo!(),
        ReturnValue::Error(_) => todo!(),
        ReturnValue::SpawnLooper => todo!(),
        ReturnValue::TransactionComplete => {
          has_transaction_complete = true;
        },
        ReturnValue::DeadReply => {
          assert!(ret.is_none());
          ret = Some(Err(TransactionError::UnreachableTarget));
        }
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

impl<Mgr: Object<Mgr>> FromProxy<Mgr> for Proxy<Mgr> {
  fn from_proxy(proxy: Proxy<Mgr>) -> Result<Self, ()> {
    Ok(proxy)
  }
}

pub struct SelfMananger(pub Proxy<SelfMananger>);

impl FromProxy<SelfMananger> for SelfMananger {
  fn from_proxy(proxy: Proxy<SelfMananger>) -> Result<Self, ()> {
    Ok(SelfMananger(proxy))
  }
}

impl Object<SelfMananger> for SelfMananger {
  fn do_transaction<'packet, 'runtime>(&self, packet: &'packet Packet<'runtime, SelfMananger>) -> Result<Packet<'runtime, SelfMananger>, TransactionError> {
    self.0.do_transaction(packet)
  }
}

