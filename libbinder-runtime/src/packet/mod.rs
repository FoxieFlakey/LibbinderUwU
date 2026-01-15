use std::sync::{Arc, atomic::Ordering};

use crate::{ArcRuntime, object::{self, Object}, packet::{builder::PacketBuilder, reader::Reader}};

pub mod reader;
pub mod writer;
pub mod builder;

use enumflags2::BitFlags;
pub use libbinder::formats::*;
pub use libbinder_raw::transaction::TransactionFlag;
use libbinder_raw::types::reference::ObjectRef;

#[derive(Clone)]
pub struct Packet<'runtime, Mgr: Object<Mgr>> {
  pub(crate) runtime: &'runtime ArcRuntime<Mgr>,
  pub(crate) packet: libbinder::packet::Packet<'runtime>,
  refs: Vec<(ObjectRef, Option<Arc<dyn Object<Mgr>>>)>
}

impl<'packet, 'runtime: 'packet, Mgr: Object<Mgr>> Packet<'runtime, Mgr> {
  pub fn new(runtime: &'runtime ArcRuntime<Mgr>, packet: libbinder::packet::Packet<'runtime>) -> Self {
    let mut refs = Vec::new();
    
    for (_, kernel_ref) in packet.iter_references() {
      let obj = match kernel_ref {
        ObjectRef::Local(local) => Some(unsafe { object::from_local_ref::<Mgr>(local) }),
        ObjectRef::Remote(remote_ref) => {
          // If remote, we increment one
          runtime.____rt.remote_reference_counters.read()
            .unwrap()
            .get(&remote_ref)
            .unwrap()
            .fetch_add(1, Ordering::Relaxed);
          
          None
        }
      };
      refs.push((kernel_ref, obj));
    }
    
    Packet {
      runtime: runtime,
      packet,
      refs
    }
  }
  
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
      builder: self.packet.into(),
      _kept_refs: self.refs
    }
  }
  
  pub fn iter_references(&self) -> impl Iterator<Item = (usize, ObjectRef)> {
    self.packet.iter_references()
  }
  
  pub fn get_code(&self) -> u32 {
    self.packet.get_code()
  }
  
  pub fn get_flags(&self) -> BitFlags<TransactionFlag> {
    self.packet.get_flags()
  }
}

