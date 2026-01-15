use libbinder_runtime::{object::{FromProxy, Object, TransactionError}, packet::{Packet, TransactionFlag, dead_simple::{DeadSimpleFormat, DeadSimpleFormatReader}}, proxy::Proxy};

use crate::interface::{self, IObject, service_manager::{self, IServiceManager}};

pub struct IServiceManagerProxy {
  proxy: Proxy<IServiceManagerProxy>
}

impl FromProxy<IServiceManagerProxy> for IServiceManagerProxy {
  fn from_proxy(proxy: Proxy<IServiceManagerProxy>) -> Result<Self, ()> {
    if interface::is_implemented(&proxy, service_manager::INTERFACE_ID).map_err(|_| ())? {
      Ok(Self { proxy })
    } else {
      Err(())
    }
  }
}

// Potentially can be compile time generated!
impl IObject for IServiceManagerProxy {
  fn is_implemented(&self, interface_id: u64) -> Result<bool, TransactionError> {
    interface::is_implemented(&self.proxy, interface_id)
  }
}

impl IServiceManager for IServiceManagerProxy {
  fn print(&self, data: &str) -> Result<(), TransactionError> {
    let rt = self.proxy.get_runtime();
    
    let mut packet = rt.new_packet();
    packet.set_code(service_manager::PRINT);
    packet.writer(DeadSimpleFormat::new())
      .write_str(data);
    let packet = packet.build();
    
    let response = self.do_transaction(&packet)?.expect("This is not oneway transaction");
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
  
  fn oneway_print(&self, data: &str) -> Result<(), TransactionError> {
    let rt = self.proxy.get_runtime();
    
    let mut packet = rt.new_packet();
    
    // This one was intentionally made calling to non oneway print, to test logics
    packet.set_code(service_manager::ONEWAY_PRINT);
    packet.set_flags(TransactionFlag::OneWay.into());
    packet.writer(DeadSimpleFormat::new())
      .write_str(data);
    let packet = packet.build();
    
    self.do_transaction(&packet)?;
    Ok(())
  }
  
  fn stop(&self) -> Result<(), TransactionError> {
    let rt = self.proxy.get_runtime();
    
    let mut packet = rt.new_packet();
    packet.set_code(service_manager::STOP);
    let packet = packet.build();
    
    let response = self.do_transaction(&packet)?.expect("This is not oneway transaction");
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
  fn do_transaction<'packet, 'runtime>(&self, packet: &'packet Packet<'runtime, IServiceManagerProxy>) -> Result<Option<Packet<'runtime, IServiceManagerProxy>>, TransactionError> {
    self.proxy.do_transaction(packet)
  }
}

