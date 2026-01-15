use std::sync::Arc;

use libbinder_runtime::{ArcRuntime, object::FromProxy};

use crate::{common::log, impls, interface, proxy};

pub fn init() {
}

pub fn main() {
  impls::service_manager::wait();
  
  let binder_dev = nix::fcntl::open("/dev/binder", nix::fcntl::OFlag::O_CLOEXEC | nix::fcntl::OFlag::O_NONBLOCK | nix::fcntl::OFlag::O_RDWR, nix::sys::stat::Mode::empty()).unwrap();
  let runtime = ArcRuntime::new(binder_dev, |_, proxy| proxy::IServiceManagerProxy::from_proxy(proxy).unwrap())
    .unwrap();
  let manager = (runtime.get_manager().clone()) as Arc<dyn interface::IServiceManager>;
  
  if manager.is_implemented(interface::service_manager::ID).unwrap() {
    log!("IServerManager is implemented by manager");
  } else {
    log!("IServerManager is not implemented by manager");
  }
  
  manager.print("Hello World, sent from other process").unwrap();
  manager.print("Hello World, sent from other process").unwrap();
  manager.print("Hello World, sent from other process").unwrap();
  manager.print("Hello World, sent from other process").unwrap();
  manager.oneway_print("here is one way print").unwrap();
  manager.print("Hello World, sent from other process").unwrap();
  manager.print("Hello World, sent from other process").unwrap();
  manager.print("Hello World, sent from other process").unwrap();
  manager.stop().unwrap();
}

