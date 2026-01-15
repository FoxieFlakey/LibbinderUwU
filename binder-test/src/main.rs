#![feature(never_type)]

use std::{env, fmt::Write, process::exit, sync::atomic::{AtomicBool, Ordering}};

use nix::{sys::wait::waitpid, unistd::{ForkResult, Pid, fork}};

use common::log;

mod common;
mod process_sync;

mod interface;
mod proxy;
mod impls;

pub fn hexdump(bytes: &[u8]) {
  let (chunks, remainder) = bytes.as_chunks::<32>();
  fn dump(bytes: &[u8]) {
    let mut serialized = String::new();
    for byte in bytes {
      write!(&mut serialized, "{byte:02x} ").unwrap();
    }
    serialized.pop();
    println!("0x{:#16x} {serialized}", bytes.as_ptr().addr());
  }
  
  chunks.iter()
    .for_each(|x| dump(x));
  dump(remainder);
}

fn divide<F: FnOnce()>(on_child: F) -> Pid {
  match unsafe { fork() }.unwrap() {
    ForkResult::Child => {
      on_child();
      exit(0);
    },
    ForkResult::Parent { child } => child
  }
}

const TASKS_TO_START: [(&str, fn(), fn()); 2] = [
  ("service-manager", impls::service_manager::init, impls::service_manager::main),
  ("app", impls::app::init, impls::app::main)
];

static IS_ALONE: AtomicBool = AtomicBool::new(false);

pub fn is_alone() -> bool {
  IS_ALONE.load(Ordering::Relaxed)
}

fn main() {
  let args: Vec<String> = env::args().collect();
  for task in TASKS_TO_START.iter() {
    task.1()
  }
  
  if args.len() == 1 {
    let mut tasks = Vec::new();
    for task in TASKS_TO_START.iter() {
      tasks.push(divide(|| {
        *common::CURRENT_NAME.write().unwrap() = task.0;
        log!("Starting");
        task.2();
        log!("Stopped");
      }));
    }
    
    log!("Waiting for childs to stop");
    for pid in tasks.iter() {
      waitpid(*pid, None).unwrap();
    }
  } else if args.len() == 2 {
    let service_name = &args[1];
    let service = TASKS_TO_START.iter()
      .filter(|x| x.0 == service_name)
      .next();
    
    if let Some(task) = service {
      *common::CURRENT_NAME.write().unwrap() = task.0;
      IS_ALONE.store(true, Ordering::Relaxed);
      
      log!("Starting");
      task.2();
      log!("Stopped");
    } else {
      eprintln!("Service '{service_name}' not found");
    }
  } else {
    eprintln!("Usage: {} [service to start]", args[0]);
    eprintln!(" When given argument start that specific service");
    eprintln!(" else all will be started");
    exit(1);
  }
}

