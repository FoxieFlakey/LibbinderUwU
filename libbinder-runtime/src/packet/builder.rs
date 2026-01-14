use libbinder::formats::WriteFormat;

use crate::{ArcRuntime, object::Object, packet::{Packet, writer::Writer}};

pub struct PacketBuilder<'runtime, Mgr: Object<Mgr>> {
  pub(crate) runtime: &'runtime ArcRuntime<Mgr>,
  pub(crate) builder: libbinder::packet::builder::PacketBuilder<'runtime>
}

impl<'packet, 'runtime: 'packet, Mgr: Object<Mgr>> PacketBuilder<'runtime, Mgr> {
  pub fn get_runtime(&self) -> &'runtime ArcRuntime<Mgr> {
    &self.runtime
  }
  
  pub fn set_code(&mut self, code: u32) -> &mut Self {
    self.builder.set_code(code);
    self
  }
  
  pub fn writer<Format: WriteFormat<'packet>>(&'packet mut self, format: Format) -> Writer<'packet, 'runtime, Format, Mgr> {
    Writer {
      runtime: self.runtime,
      writer: self.builder.writer(format)
    }
  }
  
  pub fn build(mut self) -> Packet<'runtime, Mgr> {
    Packet {
      runtime: self.runtime,
      packet: self.builder.build()
    }
  }
}



