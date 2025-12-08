use std::sync::Arc;

use libbinder::{formats::dead_simple::{DeadSimpleFormat, DeadSimpleFormatReader}, packet::{Packet, PacketSendError}};
use libbinder_raw::{object::reference::ObjectRefRemote};
use libbinder_runtime::{Runtime, binder_object::{BinderObject, ConreteObjectFromRemote}};

pub trait IAnService: Send + Sync {
  fn length_of_string(&self, string: &str) -> usize;
  fn bwah_uwu(&self, data: &str);
}

pub const ISERVICE_MANAGER_RET_OK: u32 = 0x00;
pub const ISERVICE_MANAGER_RET_ERR: u32 = 0x01;
pub const ISERVICE_MANAGER_CODE_LENGTH_OF_STRING: u32 = 0x01;
pub const ISERVICE_MANAGER_CODE_BWAH_UWU: u32 = 0x02;

// Possibly can be generated at compile time doesn't need to
// hardcode. This proxies thru Binder to actual implementation
// on diff process
pub struct AnServiceProxy {
  runtime: Arc<Runtime<AnServiceProxy>>,
  remote_ref: ObjectRefRemote
}

impl AnServiceProxy {
  fn handle_err(&self, packet: &Packet) -> ! {
    assert!(packet.get_code() == ISERVICE_MANAGER_RET_ERR);
    let err_msg = packet.reader(DeadSimpleFormatReader::new()).read_str()
      .expect("Error reading error message");
    panic!("Error from remote object: {}", err_msg);
  }
}

impl IAnService for AnServiceProxy {
  fn bwah_uwu(&self, data: &str) {
    let mut builder = self.runtime.new_packet_builder();
    builder.writer(DeadSimpleFormat::new())
      .write_str(data);
    builder.set_code(ISERVICE_MANAGER_CODE_BWAH_UWU);
    
    let response = self.runtime.send_packet(self.remote_ref.clone(), &builder.build()).unwrap();
    assert!(response.get_code() == ISERVICE_MANAGER_RET_OK || response.get_code() == ISERVICE_MANAGER_RET_ERR);
    
    if response.get_code() == ISERVICE_MANAGER_RET_ERR {
      self.handle_err(&response);
    }
  }
  
  fn length_of_string(&self, string: &str) -> usize {
    let mut builder = self.runtime.new_packet_builder();
    builder.writer(DeadSimpleFormat::new())
      .write_str(string);
    builder.set_code(ISERVICE_MANAGER_CODE_LENGTH_OF_STRING);
    
    let response = self.runtime.send_packet(self.remote_ref.clone(), &builder.build()).unwrap();
    assert!(response.get_code() == ISERVICE_MANAGER_RET_OK || response.get_code() == ISERVICE_MANAGER_RET_ERR);
    
    if response.get_code() == ISERVICE_MANAGER_RET_ERR {
      self.handle_err(&response);
    }
    
    response.reader(DeadSimpleFormatReader::new())
      .read_usize()
      .unwrap()
  }
}

impl ConreteObjectFromRemote<AnServiceProxy> for AnServiceProxy {
  fn try_from_remote(runtime: &Arc<Runtime<AnServiceProxy>>, remote_ref: ObjectRefRemote) -> Result<Self, ()> {
    Ok(AnServiceProxy {
      runtime: runtime.clone(),
      remote_ref
    })
  }
}

impl BinderObject<AnServiceProxy> for AnServiceProxy {
  fn on_packet<'runtime>(&self, runtime: &'runtime Arc<Runtime<AnServiceProxy>>, packet: &Packet<'runtime>) -> Packet<'runtime> {
    match runtime.send_packet(self.remote_ref.clone(), packet) {
      Ok(reply) => reply,
      Err(PacketSendError::DeadTarget) => panic!("Target was dead cannot proxyy over"),
      Err(e) => panic!("Error occur while proxying to remote object: {e:#?}")
    }
  }
}



