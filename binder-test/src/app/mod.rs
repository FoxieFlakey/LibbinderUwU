use libbinder_runtime::{Runtime, reference::Reference};

use crate::{common::log, interface::{IAnService, AnServiceProxy}};

pub fn main() {
  let runtime = Runtime::<AnServiceProxy>::new().ok().unwrap();
  nix::unistd::sleep(1);
  
  // The client can act on IAnService interface
  // which in general context concrete type going to
  // act according if its local/remote
  
  // It is upcasted to IAnService to show that  concrete
  // type is not need and this is how pattern generally is like
  let ctx_mgr = runtime.get_context_manager().clone() as Reference<'_, AnServiceProxy, dyn IAnService>;
  ctx_mgr.bwah_uwu("Meooow this is called from other process UwU");
  let length = ctx_mgr.length_of_string("String");
  log!("Length of string is {length}");
}
