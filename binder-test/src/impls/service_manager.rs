use std::sync::{Condvar, LazyLock, Mutex};

use libbinder_runtime::{ArcRuntime, WeakRuntime, object::{Object, TransactionError}, packet::{Packet, dead_simple::{DeadSimpleFormat, DeadSimpleFormatReader}}};

use crate::{common::log, interface::{self, service_manager::{self, IServiceManager}}, process_sync::shared_completion::SharedCompletion};

static READY: LazyLock<SharedCompletion> = LazyLock::new(|| SharedCompletion::new());

pub fn wait() {
  if crate::is_alone() {
    return;
  }
  
  READY.wait_for_completion();
}

struct ServiceManager {
  stop_requested: Mutex<bool>,
  stop_wakeup: Condvar,
  runtime: WeakRuntime<ServiceManager>
}

pub fn init() {
  LazyLock::force(&READY);
}

pub fn main() {
  READY.complete();
  
  let binder_dev = nix::fcntl::open("/dev/binder", nix::fcntl::OFlag::O_CLOEXEC | nix::fcntl::OFlag::O_NONBLOCK | nix::fcntl::OFlag::O_RDWR, nix::sys::stat::Mode::empty()).unwrap();
  let rt = ArcRuntime::new_as_manager(binder_dev, ServiceManager::new)
    .unwrap();
  
  let manager = rt.get_manager();
  let mut guard = manager.stop_requested.lock().unwrap();
  while *guard == false {
    guard = manager.stop_wakeup.wait(guard).unwrap();
  }
}

impl ServiceManager {
  pub fn new(runtime: WeakRuntime<Self>) -> Self {
    Self {
      stop_requested: Mutex::new(false),
      stop_wakeup: Condvar::new(),
      runtime
    }
  }
}

impl IServiceManager for ServiceManager {
  fn print(&self, data: &str) -> Result<(), TransactionError> {
    log!("Service manager was requested to print: '{data}'");
    Ok(())
  }
  
  fn stop(&self) -> Result<(), TransactionError> {
    *self.stop_requested.lock().unwrap() = true;
    self.stop_wakeup.notify_all();
    Ok(())
  }
}

// Potentially can be compile time generated!
impl Object<ServiceManager> for ServiceManager {
  fn do_transaction<'packet, 'runtime>(&self, packet: &'packet Packet<'runtime, ServiceManager>) -> Result<Packet<'runtime, ServiceManager>, TransactionError> {
    assert!(self.runtime.ptr_eq(&packet.get_runtime().downgrade()), "attempt to process packet belonging to other runtime");
    let rt = packet.get_runtime();
    let mut reader = packet.reader(DeadSimpleFormatReader::new());
    
    match packet.get_code() {
      service_manager::PRINT => {
        let Ok(arg1) = reader.read_str() else {
          let mut response = packet.get_runtime().new_packet();
          response.set_code(interface::ERROR_REPLY);
          
          let mut writer = response.writer(DeadSimpleFormat::new());
          writer.write_str("Malformed transaction");
          drop(writer);
          
          let resp = response.build();
          return Ok(resp);
        };
        
        self.print(arg1).unwrap();
        
        let mut response = rt.new_packet();
        response.set_code(service_manager::PRINT_REPLY);
        Ok(response.build())
      }
      
      service_manager::STOP => {
        self.stop().unwrap();
        
        let mut response = rt.new_packet();
        response.set_code(service_manager::STOP_REPLY);
        Ok(response.build())
      }
      
      x => {
        let mut response = packet.get_runtime().new_packet();
        response.set_code(interface::ERROR_REPLY);
        log!("Received unknown code: 0x{x:x}");
        
        let mut writer = response.writer(DeadSimpleFormat::new());
        writer.write_str(format!("Unrecognized transaction code: 0x{x:x}").as_str());
        drop(writer);
        
        Ok(response.build())
      }
    }
  }
}


