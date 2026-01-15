use std::{borrow::Cow, cell::RefCell, mem::{self, ManuallyDrop}, os::fd::BorrowedFd};

use libbinder::{command_buffer::{Command, CommandBuffer}, packet::Packet as libbinder_Packet, return_buffer::{ReturnBuffer, ReturnValue}};
use libbinder_raw::types::reference::ObjectRefLocal;

use crate::{ArcRuntime, object::Object, boxed_object::BoxedObject, packet::Packet};

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
  pub fn exec<'data, 'runtime: 'data, F1, F2, Mgr: Object<Mgr>>(&self, runtime: &'runtime ArcRuntime<Mgr>, command_builder: F1, mut ret_handle_func: F2)
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
        cmd_buf.exec_always_block(Some(&mut ret_buf)).unwrap();
      }
      
      for ret in ret_buf.get_parsed() {
        match ret {
          ReturnValue::Transaction(transaction) => {
            queued_transactions.push((transaction.0.clone(), transaction.1.clone()));
          },
          ReturnValue::Acquire(local_ref) => {
            let obj = unsafe { BoxedObject::<Mgr>::from_raw(local_ref.clone()) };
            obj.on_bc_acquire();
          },
          ReturnValue::AcquireWeak(local_ref) => {
            let obj = unsafe { BoxedObject::<Mgr>::from_raw(local_ref.clone()) };
            obj.on_bc_increfs();
          },
          ReturnValue::Release(local_ref) => {
            let mut obj = unsafe { BoxedObject::<Mgr>::from_raw(local_ref.clone()) };
            obj.on_bc_release();
            
            if obj.is_dead() {
              // Remove from live object map
              let mut map = runtime.____rt.local_objects_sent_outside.write().unwrap();
              map.remove(local_ref)
                .expect("Local object was not in the live objects map, for some reason");
              unsafe { ManuallyDrop::drop(&mut obj) };
            }
          },
          ReturnValue::ReleaseWeak(local_ref) => {
            let mut obj = unsafe { BoxedObject::<Mgr>::from_raw(local_ref.clone()) };
            obj.on_bc_decrefs();
            
            if obj.is_dead() {
              // Remove from live object map
              let mut map = runtime.____rt.local_objects_sent_outside.write().unwrap();
              map.remove(local_ref)
                .expect("Local object was not in the live objects map, for some reason");
              unsafe { ManuallyDrop::drop(&mut obj) };
            }
          },
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
          let obj = unsafe { BoxedObject::<Mgr>::from_raw(obj.clone()) };
          let packet = Packet { runtime: runtime, packet };
          let reply = obj.get_object().do_transaction(&packet).unwrap();
          
          let mut cmd_buf = CommandBuffer::new(runtime.get_binder());
          
          cmd_buf.enqueue_command(Command::SendReply(Cow::Borrowed(&reply.packet)))
            .exec_always_block(None)
            .unwrap();
          
          // print!("Responded");
        }
        
        queued_transactions.clear();
        // Loop back again to check new entry
      }
    }
  }
}

