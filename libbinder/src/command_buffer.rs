use std::{io, marker::PhantomData, os::fd::{AsFd, BorrowedFd}};

use libbinder_raw::{Command as CommandRaw, Transaction, binder_read_write};
use nix::{errno::Errno, poll::{PollFd, PollFlags, PollTimeout, poll}};

use crate::return_buffer::ReturnBuffer;

pub enum Command<'binder, 'data> {
  EnterLooper,
  ExitLooper,
  SendTransaction(Transaction<'binder, 'data, 'data>),
  SendReply(Transaction<'binder, 'data, 'data>),
  RegisterLooper
}

pub struct CommandBuffer<'binder, 'data> {
  binder_dev: BorrowedFd<'binder>,
  buffer: Vec<u8>,
  commands_end_offsets: Vec<usize>,
  _phantom: PhantomData<Command<'binder, 'data>>
}

pub enum ExecResult {
  // All executed normally, with no EAGAIN
  Ok,
  
  // The would block variants only occur
  // if binder_dev is opened as O_NONBLOCK
  
  // All commands were executed but the
  // read side would block
  WouldBlockOnRead,
  
  // Some commands were executed and exec
  // would block
  //
  // Contains number of of commands that
  // was executed. Again commands are
  // executed sequentially so 2 executed
  // means 0 and 1 is executed and if
  // there 3rd command. then the third
  // command would block
  //
  // Alternative the field can be thought
  // as resume index. Which can be passed
  // to 'exec' to try resume again
  WouldBlockOnWrite(usize)
}

impl ExecResult {
  pub fn panic_if_blocking(&self) {
    match self {
      ExecResult::Ok => (),
      ExecResult::WouldBlockOnRead | ExecResult::WouldBlockOnWrite(_) => panic!("Must not block")
    }
  }
}

impl<'binder, 'data> CommandBuffer<'binder, 'data> {
  pub fn new(binder_dev: BorrowedFd<'binder>) -> Self {
    Self {
      buffer: Vec::new(),
      commands_end_offsets: Vec::new(),
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
    
    self.commands_end_offsets.push(self.buffer.len());
    self
  }
  
  fn find_cmd_idx_from_bytes_written(&self, written_bytes: usize) -> Option<usize> {
    if written_bytes == 0 {
      return None;
    }
    
    Some(self.commands_end_offsets.binary_search(&written_bytes).unwrap())
  }
  
  fn cmd_idx_to_buffer_idx(&self, idx: usize) -> usize {
    self.commands_end_offsets[idx]
  }
  
  // On success, it return total number of commands executed potentially
  // less if a command would blocks.
  // On error, it return number of commands that is executed
  //
  // Note: the commands are always executed sequentially so
  // if 2 commands executed then commands at index 0 and 1 is
  // always already executed
  pub fn exec(&mut self, return_buf: Option<&mut ReturnBuffer<'binder>>) -> Result<ExecResult, (usize, io::Error)> {
    self.exec_impl(return_buf, None, false)
  }
  
  // Same as exec but resume from a specific command (also can be used to exec starting at specific point)
  pub fn exec_resume(&mut self, return_buf: Option<&mut ReturnBuffer<'binder>>, resume_cmd_idx: usize) -> Result<ExecResult, (usize, io::Error)> {
    self.exec_impl(return_buf, Some(self.cmd_idx_to_buffer_idx(resume_cmd_idx)), false)
  }
  
  // Same as exec but will always block, and do poll as necessary
  pub fn exec_always_block(&mut self, return_buf: Option<&mut ReturnBuffer<'binder>>) -> Result<(), (usize, io::Error)> {
    self.exec_impl(return_buf, None, true).map(|x| x.panic_if_blocking())
  }
  
  fn exec_impl(&mut self, mut return_buf: Option<&mut ReturnBuffer<'binder>>, resume_offset: Option<usize>, do_poll: bool) -> Result<ExecResult, (usize, io::Error)> {
    if let Some(buf) = return_buf.as_mut() {
      buf.clear();
    }
    let offset = resume_offset.unwrap_or(0);
    let mut write_buf = &self.buffer.as_slice()[offset..];
    let mut read_buf = return_buf.as_mut().map(|x| x.buffer.as_mut_slice()).unwrap_or(&mut []);
    let bytes_written;
    let bytes_read;
    
    'retry_loop: loop {
      if do_poll {
        // Poll loop to wait until ready
        'poll_loop: loop {
          if read_buf.len() == 0 {
            break 'poll_loop;
          }
          
          let mut fds = [
            PollFd::new(self.binder_dev.as_fd(), PollFlags::POLLIN)
          ];
          
          match poll(&mut fds, PollTimeout::NONE) {
            Ok(_) => {
              if fds[0].any().unwrap() {
                break 'poll_loop;
              }
            },
            Err(Errno::EINTR) => (),
            Err(e) => panic!("Error polling: {e}")
          }
        }
      }
      
      match binder_read_write(self.binder_dev, write_buf, read_buf) {
        Ok(x) => {
          (bytes_written, bytes_read) = x;
          break;
        }
        Err((Errno::EINTR, (bytes_written, bytes_read))) => {
          write_buf = &write_buf[bytes_written..];
          read_buf = &mut read_buf[bytes_read..];
        }
        Err((Errno::EAGAIN, (bytes_written, bytes_read))) => {
          if do_poll {
            // Treat this EAGAIN as EINTR, if do_poll is true
            write_buf = &write_buf[bytes_written..];
            read_buf = &mut read_buf[bytes_read..];
            continue 'retry_loop;
          }
          
          if let Some(buf) = return_buf {
            buf.parse(bytes_read);
          }
          
          let num_executed = self.find_cmd_idx_from_bytes_written(bytes_written + offset).map(|x| x + 1).unwrap_or(0);
          if num_executed == self.commands_end_offsets.len() {
            return Ok(ExecResult::WouldBlockOnRead);
          } else {
            return Ok(ExecResult::WouldBlockOnWrite(num_executed));
          }
        }
        Err((e, (bytes_written, bytes_read))) => {
          if let Some(buf) = return_buf {
            buf.parse(bytes_read);
          }
          
          return Err((self.find_cmd_idx_from_bytes_written(bytes_written + offset).map(|x| x + 1).unwrap_or(0), e.into()));
        }
      }
    }
    
    assert!(bytes_written + offset == self.buffer.len());
    if let Some(buf) = return_buf {
      buf.parse(bytes_read);
    }
    
    Ok(ExecResult::Ok)
  }
  
  pub fn clear<'new_data>(mut self) -> CommandBuffer<'binder, 'new_data> {
    self.buffer.clear();
    self.commands_end_offsets.clear();
    
    CommandBuffer {
      binder_dev: self.binder_dev,
      buffer: self.buffer,
      commands_end_offsets: self.commands_end_offsets,
      _phantom: PhantomData
    }
  }
}

