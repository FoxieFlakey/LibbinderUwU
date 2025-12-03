use nix::{request_code_none, request_code_read, request_code_write};
use num_enum::{TryFromPrimitive, TryFromPrimitiveError};

use crate::transaction::TransactionDataRaw;

const BINDER_CMD_MAGIC: u8 = b'c';

#[repr(i32)]
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
pub enum Command {
  SendTransaction = request_code_write!(BINDER_CMD_MAGIC, 0, size_of::<TransactionDataRaw>()),
  SendReply = request_code_write!(BINDER_CMD_MAGIC, 1, size_of::<TransactionDataRaw>()),
  EnterLooper = request_code_none!(BINDER_CMD_MAGIC, 12),
  ExitLooper = request_code_none!(BINDER_CMD_MAGIC, 13),
  FreeBuffer = request_code_none!(BINDER_CMD_MAGIC, 3)
}

impl Command {
  pub fn as_bytes(self) -> [u8; 4] {
    (self as u32).to_ne_bytes()
  }
}

const BINDER_RET_MAGIC: u8 = b'r';

#[repr(i32)]
#[derive(Debug, Clone, Copy, TryFromPrimitive)]
pub enum ReturnVal {
  Error = request_code_read!(BINDER_RET_MAGIC, 0, size_of::<i32>()),
  Failed = request_code_none!(BINDER_RET_MAGIC, 17),
  TransactionComplete = request_code_none!(BINDER_RET_MAGIC, 6),
  Ok = request_code_none!(BINDER_RET_MAGIC, 1),
  Noop = request_code_none!(BINDER_RET_MAGIC, 12)
}

impl ReturnVal {
  pub fn try_from_bytes(bytes: [u8; 4]) -> Result<Self, TryFromPrimitiveError<Self>> {
    Self::try_from_primitive(i32::from_ne_bytes(bytes))
  }
}


