// This code is reserved for error on all methods call
pub const ERROR_REPLY: u32 = 0x000;

pub mod service_manager {
  use libbinder_runtime::object::TransactionError;
  
  pub const STOP: u32 = 0x100;
  pub const STOP_REPLY: u32 = 0x100;
  
  pub const PRINT: u32 = 0x101;
  pub const PRINT_REPLY: u32 = 0x100;
  
  pub trait IServiceManager {
    fn print(&self, data: &str) -> Result<(), TransactionError>;
    fn stop(&self) -> Result<(), TransactionError>;
  }
}


