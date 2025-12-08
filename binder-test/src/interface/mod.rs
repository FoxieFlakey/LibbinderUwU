use std::sync::Arc;

use libbinder_runtime::{Runtime, binder_object::{BinderObject, CreateProxyFromRemote}, packet::Packet, proxy::ProxyObject, formats::dead_simple::{DeadSimpleFormat, DeadSimpleFormatReader}};

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
  proxy: ProxyObject<AnServiceProxy>
}

impl AnServiceProxy {
  fn handle_err<ContextMananger: BinderObject<ContextMananger>>(&self, packet: &Packet<ContextMananger>) -> ! {
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
    
    let response = self.on_packet(&self.runtime, &builder.build());
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
    
    let response = self.on_packet(&self.runtime, &builder.build());
    assert!(response.get_code() == ISERVICE_MANAGER_RET_OK || response.get_code() == ISERVICE_MANAGER_RET_ERR);
    
    if response.get_code() == ISERVICE_MANAGER_RET_ERR {
      self.handle_err(&response);
    }
    
    response.reader(DeadSimpleFormatReader::new())
      .read_usize()
      .unwrap()
  }
}

impl CreateProxyFromRemote<Self> for AnServiceProxy {
  fn try_from_remote(runtime: &Arc<Runtime<Self>>, remote_ref: ProxyObject<Self>) -> Result<Self, ()> {
    Ok(Self {
      runtime: runtime.clone(),
      proxy: remote_ref
    })
  }
}

impl BinderObject<Self> for AnServiceProxy {
  fn on_packet<'runtime>(&self, runtime: &'runtime Arc<Runtime<Self>>, packet: &Packet<'runtime, Self>) -> Packet<'runtime, Self> {
    self.proxy.on_packet(runtime, packet)
  }
}



