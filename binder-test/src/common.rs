use nix::unistd;

pub fn log_impl(str: &str) {
  let mut buffer = String::new();
  buffer.push_str(&format!("[{}] {str}", unistd::getpid()));
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

