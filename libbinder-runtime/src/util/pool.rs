use std::{ops::{Deref, DerefMut}, sync::Mutex};

pub struct Handle<'a, T>(&'a Pool<T>, T);

impl<'pool, T> Handle<'pool, T> {
  pub fn into(self) -> T {
    self.1
  }
  
  pub fn add_to_pool(pool: &'pool Pool<T>, other: T) -> Self {
    Self(pool, other)
  }
}

impl<T> Deref for Handle<'_, T> {
  type Target = T;
  
  fn deref(&self) -> &Self::Target {
    &self.1
  }
}

impl<T> DerefMut for Handle<'_, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.1
  }
}

pub struct Pool<T> {
  creator: Box<dyn Fn() -> T + Send + Sync>,
  pool: Mutex<Vec<T>>
}

impl<T> Pool<T> {
  pub fn new(creator: impl Fn() -> T + Send + Sync + 'static) -> Self {
    Self {
      pool: Mutex::new(Vec::new()),
      creator: Box::new(creator) as Box<dyn Fn() -> T + Send + Sync>
    }
  }
  
  pub fn get<'a>(&'a self) -> Handle<'a, T> {
    let ret = self.pool.lock().unwrap().pop();
    if let Some(x) = ret {
      Handle(self, x)
    } else {
      Handle(self, (self.creator)())
    }
  }
}


