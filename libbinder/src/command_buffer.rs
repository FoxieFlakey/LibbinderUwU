use std::{marker::PhantomData, os::fd::BorrowedFd};

use libbinder_raw::{Command as CommandRaw, Transaction, binder_read_write};

use crate::return_buffer::ReturnBuffer;

pub enum Command<'binder, 'data> {
  EnterLooper,
  #[expect(unused)]
  ExitLooper,
  SendTransaction(Transaction<'binder, 'data, 'data>),
  #[expect(unused)]
  SendReply(Transaction<'binder, 'data, 'data>),
  #[expect(unused)]
  RegisterLooper
}

pub struct CommandBuffer<'binder, 'data> {
  binder_dev: BorrowedFd<'binder>,
  buffer: Vec<u8>,
  _phantom: PhantomData<Command<'binder, 'data>>
}

impl<'binder, 'data> CommandBuffer<'binder, 'data> {
  pub fn new(binder_dev: BorrowedFd<'binder>) -> Self {
    Self {
      buffer: Vec::new(),
      _phantom: PhantomData {},
      binder_dev
    }
  }
  
  pub fn enqueue_command(&mut self, cmd: Command<'binder, 'data>) -> &mut Self {
    match cmd {
      Command::EnterLooper => self.buffer.extend_from_slice(&CommandRaw::EnterLooper.as_bytes()),
      Command::ExitLooper => self.buffer.extend_from_slice(&CommandRaw::ExitLooper.as_bytes()),
      Command::RegisterLooper => self.buffer.extend_from_slice(&CommandRaw::RegisterLooper.as_bytes()),
      Command::SendReply(transaction) => {
        self.buffer.extend_from_slice(&CommandRaw::SendReply.as_bytes());
        transaction.with_bytes(|x| {
          self.buffer.extend_from_slice(x);
        });
      },
      Command::SendTransaction(transaction) => {
        self.buffer.extend_from_slice(&CommandRaw::SendTransaction.as_bytes());
        transaction.with_bytes(|x| {
          self.buffer.extend_from_slice(x);
        });
      }
    }
    
    self
  }
  
  pub fn exec(&mut self, mut return_buf: Option<&mut ReturnBuffer<'binder>>) {
    if let Some(buf) = return_buf {
      buf.clear();
    }
    let read_buf = return_buf.as_mut().map(|x| x.buffer.as_mut_slice()).unwrap_or(&mut []);
    let (bytes_written, bytes_read) = binder_read_write(self.binder_dev, &self.buffer, read_buf).unwrap();
    println!("Bytes sent {bytes_written}, Bytes received {bytes_read}");
    if let Some(buf) = return_buf {
      buf.parse();
    }
    panic!();
  }
  
  #[expect(unused)]
  pub fn clear<'new_data>(self) -> CommandBuffer<'binder, 'new_data> {
    CommandBuffer {
      binder_dev: self.binder_dev,
      buffer: self.buffer,
      _phantom: PhantomData
    }
  }
}

