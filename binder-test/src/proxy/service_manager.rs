use libbinder_runtime::{object::{FromProxy, Object, TransactionError}, packet::{Packet, dead_simple::{DeadSimpleFormat, DeadSimpleFormatReader}}, proxy::Proxy};

use crate::interface::{self, service_manager::{self, IServiceManager}};

pub struct IServiceManagerProxy {
  proxy: Proxy<IServiceManagerProxy>
}

impl FromProxy<IServiceManagerProxy> for IServiceManagerProxy {
  fn from_proxy(proxy: Proxy<IServiceManagerProxy>) -> Result<Self, ()> {
    Ok(Self { proxy })
  }
}

// Potentially can be compile time generated!
impl IServiceManager for IServiceManagerProxy {
  fn print(&self, data: &str) -> Result<(), TransactionError> {
    let rt = self.proxy.get_runtime();
    
    let mut packet = rt.new_packet();
    packet.set_code(service_manager::PRINT);
    packet.writer(DeadSimpleFormat::new())
      .write_str(data);
    let packet = packet.build();
    
    let response = self.do_transaction(&packet)?;
    let mut reader = response.reader(DeadSimpleFormatReader::new());
    
    if response.get_code() != service_manager::PRINT_REPLY {
      if response.get_code() == interface::ERROR_REPLY {
        if let Ok(error_msg) = reader.read_str() {
          return Err(TransactionError::RemoteError(Box::new(format!("Remote error: {}", error_msg))));
        }
      }
      return Err(TransactionError::MalformedReply);
    }
    
    Ok(())
  }
  
  fn stop(&self) -> Result<(), TransactionError> {
    let rt = self.proxy.get_runtime();
    
    let mut packet = rt.new_packet();
    packet.set_code(service_manager::STOP);
    let packet = packet.build();
    
    let response = self.do_transaction(&packet)?;
    let mut reader = response.reader(DeadSimpleFormatReader::new());
    
    if response.get_code() != service_manager::STOP_REPLY {
      if response.get_code() == interface::ERROR_REPLY {
        if let Ok(error_msg) = reader.read_str() {
          return Err(TransactionError::RemoteError(Box::new(format!("Remote error: {}", error_msg))));
        }
      }
      return Err(TransactionError::MalformedReply);
    }
    
    Ok(())
  }
}

impl Object<IServiceManagerProxy> for IServiceManagerProxy {
  fn do_transaction<'packet, 'runtime>(&self, packet: &'packet Packet<'runtime, IServiceManagerProxy>) -> Result<Packet<'runtime, IServiceManagerProxy>, TransactionError> {
    self.proxy.do_transaction(packet)
  }
}

