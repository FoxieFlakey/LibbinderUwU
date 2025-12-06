use libbinder_runtime::Runtime;

use crate::{common::log, interface::{IServiceManager, RemoteServiceManager}};

pub fn main() {
  let runtime = Runtime::<RemoteServiceManager>::new().ok().unwrap();
  nix::unistd::sleep(1);
  
  let ctx_mgr = &**runtime.get_context_manager() as &dyn IServiceManager;
  ctx_mgr.bwah_uwu("Yoooo this is called from other process UwU");
  let length = ctx_mgr.length_of_string("String");
  log!("Length of string is {length}");
}
