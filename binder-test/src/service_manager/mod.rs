use std::sync::Arc;

use libbinder::{formats::dead_simple::{DeadSimpleFormat, DeadSimpleFormatReader}, packet::{Packet, builder::PacketBuilder}};
use libbinder_runtime::{Runtime, binder_object::BinderObject};

use crate::{common::log, interface::{ISERVICE_MANAGER_CODE_BWAH_UWU, ISERVICE_MANAGER_CODE_LENGTH_OF_STRING, ISERVICE_MANAGER_RET_ERR, ISERVICE_MANAGER_RET_OK, IServiceManager}};

struct ContextManager;

impl IServiceManager for ContextManager {
  fn length_of_string(&self, string: &str) -> usize {
    log!("Length of string is {}", string.len());
    string.len()
  }
  
  fn bwah_uwu(&self, data: &str) {
    log!("Bwah_uwu: {}", data);
  }
}

impl BinderObject<ContextManager> for ContextManager {
  fn on_packet(&self, _runtime: &Arc<Runtime<ContextManager>>, packet: &Packet<'_>, reply_builder: &mut PacketBuilder) {
    reply_builder.set_code(ISERVICE_MANAGER_RET_OK);
    match packet.get_code() {
      ISERVICE_MANAGER_CODE_LENGTH_OF_STRING => {
        let mut reader = packet.reader(DeadSimpleFormatReader::new());
        let Ok(data) = reader.read_str() else {
          reply_builder.set_code(ISERVICE_MANAGER_RET_ERR)
            .writer(DeadSimpleFormat::new())
            .write_str("invalid string");
          return;
        };
        
        let len = self.length_of_string(data);
        reply_builder.writer(DeadSimpleFormat::new())
          .write_usize(len);
      }
      ISERVICE_MANAGER_CODE_BWAH_UWU => {
        let mut reader = packet.reader(DeadSimpleFormatReader::new());
        let Ok(data) = reader.read_str() else {
          reply_builder.set_code(ISERVICE_MANAGER_RET_ERR)
            .writer(DeadSimpleFormat::new())
            .write_str("invalid string");
          return;
        };
        self.bwah_uwu(data);
      }
      _ => {
        log!("Received unknown transaction code: {}", packet.get_code());
        reply_builder.set_code(ISERVICE_MANAGER_RET_ERR)
          .writer(DeadSimpleFormat::new())
          .write_str("unknown transaction code");
      }
    }
  }
}

pub fn main() {
  let runtime = Runtime::new_as_manager(Arc::new(ContextManager))
    .ok()
    .unwrap();
  
  let ctx = runtime.get_context_manager(); 
  ctx.bwah_uwu("HI UwU");
  loop { nix::unistd::sleep(1); }
}

