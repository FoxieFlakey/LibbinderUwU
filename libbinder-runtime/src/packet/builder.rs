use std::sync::Arc;

use libbinder::formats::WriteFormat;
use libbinder_raw::types::reference::ObjectRef;

use crate::{ArcRuntime, object::Object, packet::{Packet, writer::Writer}};

pub struct PacketBuilder<'runtime, Mgr: Object<Mgr>> {
  pub(crate) runtime: &'runtime ArcRuntime<Mgr>,
  pub(crate) builder: libbinder::packet::builder::PacketBuilder<'runtime>,
  
  // This are living reference that must be kept
  // this is non empty, if packet builder was made
  // from packet which has some references inside
  pub(super) _kept_refs: Vec<(ObjectRef, Option<Arc<dyn Object<Mgr>>>)>
}

impl<'packet, 'runtime: 'packet, Mgr: Object<Mgr>> PacketBuilder<'runtime, Mgr> {
  pub(crate) fn new(runtime: &'runtime ArcRuntime<Mgr>) -> Self {
    Self {
      builder: libbinder::packet::builder::PacketBuilder::new(runtime.get_binder()),
      runtime,
      _kept_refs: Vec::new()
    }
  }
  
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
    Packet::new(self.runtime, self.builder.build())
  }
}



