// A shared event which is for triggering an
// event which will be listened by multiple
// processes

use crate::process_sync::{shared_mutex::SharedMutex, shared_condvar::SharedCondvar};

// This is the same idea as Linux kernel's completion
// for simple waiting other process to reach a point
pub struct SharedCompletion {
  is_completed: SharedMutex<bool>,
  cond: SharedCondvar
}

impl SharedCompletion {
  pub fn new() -> SharedCompletion {
    SharedCompletion {
      is_completed: SharedMutex::new(false),
      cond: SharedCondvar::new()
    }
  }
  
  pub fn complete(&self) {
    *self.is_completed.lock() = true;
    self.cond.notify_all();
  }
  
  pub fn wait_for_completion(&self) {
    let mut is_completed = self.is_completed.lock();
    while *is_completed == false {
      self.cond.wait(&mut is_completed);
    }
  }
}

