use std::sync::{Arc, Condvar, Mutex};

use libbinder_runtime::{ArcRuntime, WeakRuntime, object::{FromProxy, Object}};

use crate::{impls, interface, proxy};

pub fn init() {
}

struct Calculator<Mgr: Object<Mgr> + ?Sized> {
  stop_requested: Mutex<bool>,
  stop_wakeup: Condvar,
  runtime: WeakRuntime<Mgr>
}

impl<Mgr: Object<Mgr> + ?Sized> interface::ICalculator for Calculator<Mgr> {
  fn add(&self, lhs: f32, rhs: f32) -> Result<f32, libbinder_runtime::object::TransactionError> {
    Ok(lhs + rhs)
  }

  fn sub(&self, lhs: f32, rhs: f32) -> Result<f32, libbinder_runtime::object::TransactionError> {
    Ok(lhs - rhs)
  }

  fn mul(&self, lhs: f32, rhs: f32) -> Result<f32, libbinder_runtime::object::TransactionError> {
    Ok(lhs * rhs)
  }

  fn div(&self, lhs: f32, rhs: f32) -> Result<f32, libbinder_runtime::object::TransactionError> {
    Ok(lhs / rhs)
  }
}

impl<Mgr: Object<Mgr> + ?Sized> interface::IService for Calculator<Mgr> {
  fn get_name(&self) -> Result<String, libbinder_runtime::object::TransactionError> {
    Ok(String::from("Calculator Service"))
  }
  
  fn stop(&self) -> Result<(), libbinder_runtime::object::TransactionError> {
    *self.stop_requested.lock().unwrap() = true;
    self.stop_wakeup.notify_all();
    
    Ok(())
  }
}

impl<Mgr: Object<Mgr> + ?Sized> interface::IObject for Calculator<Mgr> {
  fn is_implemented(&self, interface_id: u64) -> Result<bool, libbinder_runtime::object::TransactionError> {
    match interface_id {
      interface::service::ID => Ok(true),
      interface::calculator::ID => Ok(true),
      _ => Ok(false)
    }
  }
}

pub fn main() {
  impls::service_manager::wait();
  
  let binder_dev = nix::fcntl::open("/dev/binder", nix::fcntl::OFlag::O_CLOEXEC | nix::fcntl::OFlag::O_NONBLOCK | nix::fcntl::OFlag::O_RDWR, nix::sys::stat::Mode::empty()).unwrap();
  let runtime = ArcRuntime::new(binder_dev, |_, proxy| proxy::IServiceManagerProxy::from_proxy(proxy).unwrap())
    .unwrap();
  let manager = (runtime.get_manager().clone()) as Arc<dyn interface::IServiceManager>;
  
  let calculator = Arc::new(Calculator {
    runtime: runtime.downgrade(),
    stop_requested: Mutex::new(false),
    stop_wakeup: Condvar::new()
  });
  manager.register("calculator", calculator).unwrap();
  
  runtime.stop_background_threads();
}