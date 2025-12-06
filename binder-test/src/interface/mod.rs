use std::sync::Arc;

use libbinder::{formats::dead_simple::{DeadSimpleFormat, DeadSimpleFormatReader}, packet::{Packet, PacketSendError, builder::PacketBuilder}};
use libbinder_raw::ObjectRefRemote;
use libbinder_runtime::{Runtime, binder_object::{BinderObject, ConreteObjectFromRemote}};

pub trait IServiceManager: Send + Sync {
  fn length_of_string(&self, string: &str) -> usize;
  fn bwah_uwu(&self, data: &str);
}

pub const ISERVICE_MANAGER_RET_OK: u32 = 0x00;
pub const ISERVICE_MANAGER_RET_ERR: u32 = 0x01;
pub const ISERVICE_MANAGER_CODE_LENGTH_OF_STRING: u32 = 0x01;
pub const ISERVICE_MANAGER_CODE_BWAH_UWU: u32 = 0x02;

pub struct RemoteServiceManager {
  runtime: Arc<Runtime<RemoteServiceManager>>,
  remote_ref: ObjectRefRemote
}

impl RemoteServiceManager {
  fn handle_err(&self, packet: &Packet) -> ! {
    assert!(packet.get_code() == ISERVICE_MANAGER_RET_ERR);
    let err_msg = packet.reader(DeadSimpleFormatReader::new()).read_str()
      .expect("Error reading error message");
    panic!("Error from remote object: {}", err_msg);
  }
}

impl IServiceManager for RemoteServiceManager {
  fn bwah_uwu(&self, data: &str) {
    let mut builder = PacketBuilder::new();
    builder.writer(DeadSimpleFormat::new())
      .write_str(data);
    builder.set_code(ISERVICE_MANAGER_CODE_BWAH_UWU);
    
    let response = self.runtime.send_packet(self.remote_ref.clone(), &builder.build(self.runtime.get_binder())).unwrap();
    assert!(response.get_code() == ISERVICE_MANAGER_RET_OK || response.get_code() == ISERVICE_MANAGER_RET_ERR);
    
    if response.get_code() == ISERVICE_MANAGER_RET_ERR {
      self.handle_err(&response);
    }
  }
  
  fn length_of_string(&self, string: &str) -> usize {
    let mut builder = PacketBuilder::new();
    builder.writer(DeadSimpleFormat::new())
      .write_str(string);
    builder.set_code(ISERVICE_MANAGER_CODE_LENGTH_OF_STRING);
    
    let response = self.runtime.send_packet(self.remote_ref.clone(), &builder.build(self.runtime.get_binder())).unwrap();
    assert!(response.get_code() == ISERVICE_MANAGER_RET_OK || response.get_code() == ISERVICE_MANAGER_RET_ERR);
    
    if response.get_code() == ISERVICE_MANAGER_RET_ERR {
      self.handle_err(&response);
    }
    
    response.reader(DeadSimpleFormatReader::new())
      .read_usize()
      .unwrap()
  }
}

impl ConreteObjectFromRemote<RemoteServiceManager> for RemoteServiceManager {
  fn try_from_remote(runtime: &Arc<Runtime<RemoteServiceManager>>, remote_ref: ObjectRefRemote) -> Result<Self, ()> {
    Ok(RemoteServiceManager {
      runtime: runtime.clone(),
      remote_ref
    })
  }
}

impl BinderObject<RemoteServiceManager> for RemoteServiceManager {
  fn on_packet(&self, runtime: &Arc<Runtime<RemoteServiceManager>>, packet: &libbinder::packet::Packet<'_>, reply_builder: &mut libbinder::packet::builder::PacketBuilder) {
    match runtime.send_packet(self.remote_ref.clone(), packet) {
      Ok(reply) => *reply_builder = reply.into(),
      Err(PacketSendError::DeadTarget) => panic!("Target was dead cannot proxyy over"),
      Err(e) => panic!("Error occur while proxying to remote object: {e:#?}")
    }
  }
}



