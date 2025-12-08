use std::sync::Arc;

use libbinder::formats::dead_simple::{DeadSimpleFormat, DeadSimpleFormatReader};
use libbinder_runtime::{Runtime, binder_object::BinderObject, packet::Packet, reference::Reference};

use crate::{common::log, interface::{ISERVICE_MANAGER_CODE_BWAH_UWU, ISERVICE_MANAGER_CODE_LENGTH_OF_STRING, ISERVICE_MANAGER_RET_ERR, ISERVICE_MANAGER_RET_OK, IAnService}};

struct AnServiceImpl;

impl IAnService for AnServiceImpl {
  fn length_of_string(&self, string: &str) -> usize {
    log!("Length of string is {}", string.len());
    string.len()
  }
  
  fn bwah_uwu(&self, data: &str) {
    log!("Bwah_uwu: {}", data);
  }
}

pub fn main() {
  let runtime = Runtime::new_as_manager(Arc::new(AnServiceImpl))
    .ok()
    .unwrap();
  
  // The owning process of a specific service directly calls on
  // the actual impl instead thru binder
  //
  // It is upcasted to IServiceManager to show that  concrete
  // type is not need and this is how pattern generally is like
  let ctx = runtime.get_context_manager().clone() as Reference<'_, AnServiceImpl, dyn IAnService>; 
  ctx.bwah_uwu("HI UwU");
  loop { nix::unistd::sleep(1); }
}

// Possibly can be generated at compile time  doesn't need to
// hardcode. This handles the coming calls, parse and dispatch it
// to the actual implementation
impl BinderObject<AnServiceImpl> for AnServiceImpl {
  fn on_packet<'runtime>(&self, runtime: &'runtime Arc<Runtime<AnServiceImpl>>, packet: &Packet<'runtime, AnServiceImpl>) -> Packet<'runtime, AnServiceImpl> {
    let mut reply_builder = runtime.new_packet_builder();
    reply_builder.set_code(ISERVICE_MANAGER_RET_OK);
    match packet.get_code() {
      ISERVICE_MANAGER_CODE_LENGTH_OF_STRING => {
        let mut reader = packet.reader(DeadSimpleFormatReader::new());
        let Ok(data) = reader.read_str() else {
          reply_builder.set_code(ISERVICE_MANAGER_RET_ERR)
            .writer(DeadSimpleFormat::new())
            .write_str("invalid string");
          return reply_builder.build();
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
          return reply_builder.build();
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
    
    reply_builder.build()
  }
}


