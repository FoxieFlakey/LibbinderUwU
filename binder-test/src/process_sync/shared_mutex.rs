// A shared mutex for serializing access from multiple
// forked processes but won't work well with entirely
// different process!

use std::{ops::{Deref, DerefMut}, sync::{Mutex, MutexGuard}};

use process_sync::{SharedMemoryObject, SharedMutex as ProcessSyncSharedMutex};

// Requires Copy because complex objects
// can't be safely placed in shared memory
// due it might contain smart pointers
// which points to other things which other
// process might not have!
pub(super) struct State<T: Copy + Send + Sync> {
  pub region: SharedMemoryObject<T>,
  pub process_mutex: ProcessSyncSharedMutex
}

// SAFETY: Properly synchronized
unsafe impl<T: Copy + Send + Sync> Send for State<T> {}

// SAFETY: Properly synchronized
unsafe impl<T: Copy + Send + Sync> Sync for State<T> {}

pub struct SharedMutex<T:Copy + Send + Sync> {
  // An additional wrapping Mutex to synchronize access
  // to process shared mutex from other thread
  // in same process
  mutex: Mutex<State<T>>
}

pub struct SharedMutexGuard<'a, T: Copy + Send + Sync> {
  state_guard: MutexGuard<'a, State<T>>
}

impl<T: Copy + Send + Sync> SharedMutexGuard<'_, T> {
  pub(super) fn get_state_mut(&mut self) -> &mut State<T> {
    &mut self.state_guard
  }
}

impl<T: Copy + Send + Sync> Deref for SharedMutexGuard<'_, T> {
  type Target = T;
  
  fn deref(&self) -> &Self::Target {
    // SAFETY: The access is protected by shared process lock
    // and local mutex lock from being accessed by other process
    // and local thread
    unsafe { self.state_guard.region.get() }
  }
}

impl<T: Copy + Send + Sync> DerefMut for SharedMutexGuard<'_, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    // SAFETY: The access is protected by shared process lock
    // and local mutex lock from being accessed by other process
    // and local thread
    unsafe { self.state_guard.region.get_mut() }
  }
}

impl<T: Copy + Send + Sync> Drop for SharedMutexGuard<'_, T> {
  fn drop(&mut self) {
    self.state_guard.process_mutex.unlock().unwrap();
  }
}

impl<T:Copy + Send + Sync> SharedMutex<T> {
  pub fn new(data: T) -> Self {
    Self {
      mutex: Mutex::new(State {
        process_mutex: ProcessSyncSharedMutex::new().unwrap(),
        region: SharedMemoryObject::new(data).unwrap()
      })
    }
  }
  
  pub fn lock<'a>(&'a self) -> SharedMutexGuard<'a, T> {
    let mut state = self.mutex.lock().unwrap();
    
    // Will be unlocked later in SharedMutexGuard's drop code
    state.process_mutex.lock().unwrap();
    
    SharedMutexGuard {
      state_guard: state
    }
  }
}


