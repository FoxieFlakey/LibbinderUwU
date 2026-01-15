use libbinder_runtime::{object::{Object, TransactionError}, packet::dead_simple::{DeadSimpleFormat, DeadSimpleFormatReader}, proxy::Proxy};

// This code is reserved for error on all methods call
pub const ERROR_REPLY: u32 = 0x000;

// Takes one u64 argument
pub const IS_IMPLEMENTED: u32 = 0x001;
pub const IS_IMPLEMENTED_REPLY_YES: u32 = 0x100;
pub const IS_IMPLEMENTED_REPLY_NO: u32 = 0x101;

pub trait IObject {
  fn is_implemented(&self, interface_id: u64) -> Result<bool, TransactionError>;
}

pub mod service_manager {
  use libbinder_runtime::object::TransactionError;
  use crate::interface::IObject;
  
  pub const INTERFACE_ID: u64 = 0;
  
  pub const STOP: u32 = 0x100;
  pub const STOP_REPLY: u32 = 0x100;
  
  // Takes one &str argument
  pub const PRINT: u32 = 0x101;
  pub const PRINT_REPLY: u32 = 0x100;
  
  // One way version of above method
  pub const ONEWAY_PRINT: u32 = 0x102;
  
  pub trait IServiceManager: IObject {
    fn print(&self, data: &str) -> Result<(), TransactionError>;
    /* oneway */ fn oneway_print(&self, data: &str) -> Result<(), TransactionError>;
    fn stop(&self) -> Result<(), TransactionError>;
  }
}

pub fn is_implemented<Mgr: Object<Mgr>>(object: &Proxy<Mgr>, interface_id: u64) -> Result<bool, TransactionError> {
  let rt = object.get_runtime();
  let mut packet = rt.new_packet();
  packet.set_code(IS_IMPLEMENTED);
  packet.writer(DeadSimpleFormat::new())
    .write_u64(interface_id);
  
  let packet = packet.build();
  let ret = object.do_transaction(&packet)?.expect("This is not oneway transaction");
  let mut reader = ret.reader(DeadSimpleFormatReader::new());
  
  match ret.get_code() {
    ERROR_REPLY => {
      if let Ok(error_message) = reader.read_str() {
        return  Err(TransactionError::RemoteError(Box::new(format!("Remote error: {error_message}"))));
      }
      
      Err(TransactionError::MalformedReply)
    }
    
    IS_IMPLEMENTED_REPLY_YES => {
      Ok(true)
    }
    
    IS_IMPLEMENTED_REPLY_NO => {
      Ok(false)
    }
    
    _ => {
      Err(TransactionError::MalformedReply)
    }
  }
}
