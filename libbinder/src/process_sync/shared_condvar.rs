use std::sync::Mutex;

use process_sync::SharedCondvar as ProcessSharedCondvar;

use super::shared_mutex::SharedMutexGuard;

pub struct SharedCondvar {
  // Another mutex to protect from local thread accessestha
  condvar: Mutex<ProcessSharedCondvar>
}

// SAFETY: The condvar is safe to be send and sync
unsafe impl Send for SharedCondvar {}
// SAFETY: The condvar is safe to be send and sync
unsafe impl Sync for SharedCondvar {}

impl SharedCondvar {
  pub fn new() -> Self {
    Self {
      condvar: Mutex::new(ProcessSharedCondvar::new().unwrap())
    }
  }
  
  pub fn notify_all(&self) {
    self.condvar.lock().unwrap().notify_all().unwrap();
  }
  
  pub fn wait<T: Copy + Send + Sync>(&self, guard: &mut SharedMutexGuard<T>) {
    self.condvar.lock().unwrap().wait(&mut SharedMutexGuard::get_state_mut(guard).process_mutex).unwrap();
  }
}

