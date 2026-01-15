use std::sync::RwLock;

use nix::unistd;

pub static CURRENT_NAME: RwLock<&str> = RwLock::new("Bootstrap");

pub fn log_impl(str: &str) {
  let mut buffer = String::new();
  buffer.push_str(&format!("[{}] [{}] {str}", CURRENT_NAME.read().unwrap(), unistd::getpid()));
  buffer.push_str("\n");
  
  print!("{}", buffer);
}

macro_rules! log {
  () => {
    $crate::common::log_impl("");
  };
  
  ($($arg:tt)*) => {{
    $crate::common::log_impl(&format!($($arg)*));
  }};
}
pub(crate) use log;

