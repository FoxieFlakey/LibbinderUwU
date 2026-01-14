use std::{any::Any, error::Error};
use crate::packet::Packet;

#[derive(Debug)]
pub enum TransactionError {
  // The target of reply/transaction, no longer exist
  DeadTarget,
  
  // Miscellanous error
  MiscellanousError(Box<dyn Error>)
}

// About storing ArcRuntime, caller should store only weak
// reference to the runtime, don't store strong reference
//
// Runtime will store the strong reference to object if its
// sent outside
pub trait Object<Mgr: Object<Mgr>>: Sync + Send + Any + 'static {
  fn do_transaction<'runtime>(&self, packet: &'runtime Packet<'runtime, Mgr>) -> Result<Packet<'runtime, Mgr>, TransactionError>;
}



