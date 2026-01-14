use crate::{ArcRuntime, object::Object, packet::{builder::PacketBuilder, reader::Reader}};

pub mod reader;
pub mod writer;
pub mod builder;

pub use libbinder::formats::*;

#[derive(Clone)]
pub struct Packet<'runtime, Mgr: Object<Mgr>> {
  pub(crate) runtime: &'runtime ArcRuntime<Mgr>,
  pub(crate) packet: libbinder::packet::Packet<'runtime>
}

impl<'packet, 'runtime: 'packet, Mgr: Object<Mgr>> Packet<'runtime, Mgr> {
  pub fn get_runtime(&self) -> &'runtime ArcRuntime<Mgr> {
    self.runtime
  }
  
  pub fn reader<Format: ReadFormat<'packet>>(&'packet self, format: Format) -> Reader<'packet, 'runtime, Format, Mgr> {
    Reader {
      runtime: self.runtime,
      reader: self.packet.reader(format)
    }
  }
  
  pub fn into_builder(self) -> PacketBuilder<'runtime, Mgr> {
    PacketBuilder {
      runtime: self.runtime,
      builder: self.packet.into()
    }
  }
}

