use std::{borrow::Cow, cell::RefCell, mem::{self, ManuallyDrop}, os::fd::BorrowedFd, sync::Arc};

use libbinder::{command_buffer::{Command, CommandBuffer}, packet::Packet as libbinder_Packet, return_buffer::{ReturnBuffer, ReturnValue}};
use libbinder_raw::{transaction::TransactionFlag, types::reference::ObjectRefLocal};

use crate::{ArcRuntime, object::{self, Object}, packet::Packet};

struct Session {
  ret_buf: (Vec<ReturnValue<'static>>, Vec<u8>),
  cmd_buf: (Vec<u8>, Vec<usize>),
  queued_transactions: Vec<(ObjectRefLocal, libbinder_Packet<'static>)>
}

pub struct Context {
  bufs: RefCell<Option<Session>>
}

const RET_BUF_SIZE: usize = 8 * 1024 * 1024;

impl Context {
  pub fn new(binder_dev: BorrowedFd<'_>) -> Self {
    Self {
      bufs: RefCell::new(Some(Session {
        ret_buf: ReturnBuffer::new(binder_dev, RET_BUF_SIZE).into_buffers(),
        cmd_buf: CommandBuffer::new(binder_dev).into_buffers(),
        queued_transactions: Vec::new()
      }))
    }
  }
  
  // ret_handle_func is the function which handle individual return value
  pub fn exec<'data, 'runtime: 'data, F1, F2, Mgr: Object<Mgr> + ?Sized>(&self, runtime: &'runtime ArcRuntime<Mgr>, command_builder: F1, ret_handle_func: F2)
    where F1: FnOnce(&mut CommandBuffer<'runtime, 'data>),
      F2: FnMut(&ReturnValue<'runtime>)
  {
    self.exec_impl(runtime, command_builder, ret_handle_func, true);
  }
  
  pub fn exec_without_ret<'data, 'runtime: 'data, F1, Mgr: Object<Mgr> + ?Sized>(&self, runtime: &'runtime ArcRuntime<Mgr>, command_builder: F1)
    where F1: FnOnce(&mut CommandBuffer<'runtime, 'data>)
  {
    self.exec_impl(runtime, command_builder, |_| panic!("unexpected"), false);
  }
  
  fn exec_impl<'data, 'runtime: 'data, F1, F2, Mgr: Object<Mgr> + ?Sized>(&self, runtime: &'runtime ArcRuntime<Mgr>, command_builder: F1, mut ret_handle_func: F2, with_ret: bool)
    where F1: FnOnce(&mut CommandBuffer<'runtime, 'data>),
      F2: FnMut(&ReturnValue<'runtime>)
  {
    let mut is_initial = true;
    let mut builder = Some(command_builder);
    
    loop {
      let session = self.bufs.borrow_mut().take().unwrap();
      let mut ret_buf: ReturnBuffer<'runtime> = ReturnBuffer::from_buffers(runtime.get_binder(), session.ret_buf);
      let mut cmd_buf: CommandBuffer<'runtime, 'data> = CommandBuffer::from_buffers(runtime.get_binder(), session.cmd_buf);
      let mut queued_transactions: Vec<(ObjectRefLocal, libbinder_Packet<'runtime>)> = unsafe { std::mem::transmute(session.queued_transactions) };
      
      // Run initial commands
      if is_initial {
        (builder.take().unwrap())(&mut cmd_buf);
        
        if with_ret {
          cmd_buf.exec_always_block(Some(&mut ret_buf)).unwrap();
        } else {
          cmd_buf.exec_always_block(None).unwrap();
        }
      }
      
      for ret in ret_buf.get_parsed() {
        assert!(with_ret);
        
        match ret {
          ReturnValue::Transaction(transaction) => {
            queued_transactions.push((transaction.0.clone(), transaction.1.clone()));
          },
          ReturnValue::Acquire(local_ref) |
          ReturnValue::Release(local_ref) |
          ReturnValue::AcquireWeak(local_ref) |
          ReturnValue::ReleaseWeak(local_ref) => {
            let obj = unsafe { ManuallyDrop::new(object::from_local_ref::<Mgr>(local_ref.clone())) };
            let mut ref_states = runtime.____rt.reference_states.lock().unwrap();
            let ref_state = ref_states.get_mut(local_ref).expect("Unknown local object");
            let kill_object ;
            
            match ret {
              ReturnValue::Acquire(_) => {
                if ref_state.0 != false {
                  panic!("Kernel sent BC_ACQUIRE when object's strong ref count is nonzero");
                }
                ref_state.0 = true;
                kill_object = false;
              },
              ReturnValue::Release(_) => {
                if ref_state.0 != false {
                  panic!("Kernel sent BC_RELEASE when object's strong ref count is zero");
                }
                ref_state.0 = false;
                
                if ref_state.0 == false && ref_state.1 == false {
                  // Object is dead to outside, lets kill them
                  kill_object = true;
                } else {
                  kill_object = false;
                }
              },
              ReturnValue::AcquireWeak(_) => {
                if ref_state.1 != false {
                  panic!("Kernel sent BC_INCREFS when object's weak ref count is nonzero");
                }
                ref_state.1 = true;
                kill_object = false;
              },
              ReturnValue::ReleaseWeak(_) => {
                if ref_state.1 != false {
                  panic!("Kernel sent BC_DECREFS when object's weak ref count is zero");
                }
                ref_state.1 = false;
                
                if ref_state.0 == false && ref_state.1 == false {
                  // Object is dead to outside lets decrement
                  kill_object = true;
                } else {
                  kill_object = false;
                }
              },
              _ => unreachable!()
            }
            
            if kill_object {
              ref_states.remove(local_ref).unwrap();
              unsafe { Arc::decrement_strong_count(Arc::as_ptr(&obj)) };
            }
          }
          ReturnValue::Reply(_) => if is_initial { ret_handle_func(&ret) },
          ReturnValue::TransactionFailed => if is_initial { ret_handle_func(&ret) },
          ReturnValue::Ok => if is_initial { ret_handle_func(&ret) },
          ReturnValue::Error(e) => panic!("Error from binder {e}"),
          ReturnValue::SpawnLooper => (),
          ReturnValue::TransactionComplete => if is_initial { ret_handle_func(&ret) },
          ReturnValue::DeadReply => if is_initial { ret_handle_func(&ret) },
          ReturnValue::Noop => (),
        }
      }
      
      is_initial = false;
      
      if queued_transactions.is_empty() {
        // Put back the original buffers
        *self.bufs.borrow_mut() = Some(Session {
          cmd_buf: cmd_buf.into_buffers(),
          ret_buf: ret_buf.into_buffers(),
          queued_transactions: unsafe { mem::transmute(queued_transactions) }
        });
        break;
      } else {
        // Because we need the cmd_buf and ret_buf, give new
        // temporary one
        *self.bufs.borrow_mut() = Some(Session {
          cmd_buf: cmd_buf.into_buffers(),
          ret_buf: ret_buf.into_buffers(),
          queued_transactions: Vec::new()
        });
        
        // Process queue transactions after processing all return values
        for (obj, packet) in queued_transactions.drain(..) {
          let obj = ManuallyDrop::new(unsafe { object::from_local_ref(obj.clone()) });
          let packet = Packet::new(runtime, packet);
          let reply = obj.do_transaction(&packet).unwrap();
          
          let mut cmd_buf = CommandBuffer::new(runtime.get_binder());
          
          if packet.get_flags().contains(TransactionFlag::OneWay) {
            assert!(reply.is_none(), "This one way transaction!");
          } else {
            assert!(reply.is_some(), "This not oneway transaction!");
            cmd_buf.enqueue_command(Command::SendReply(Cow::Borrowed(&reply.unwrap().packet)))
              .exec_always_block(None)
              .unwrap();
          }
        }
        
        queued_transactions.clear();
        // Loop back again to check new entry
      }
    }
  }
}

