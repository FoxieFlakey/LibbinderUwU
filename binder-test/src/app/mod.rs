use libbinder_raw::{ObjectRef, ObjectRefRemote};
use libbinder_runtime::Runtime;

pub fn main() {
  let runtime = Runtime::new().unwrap();
  nix::unistd::sleep(1);
  
  let response = runtime.send_packet(
      ObjectRef::Remote(ObjectRefRemote { data_handle: 0 }),
      &runtime.new_packet()
        .set_code(80386)
        .build()
    ).unwrap();
  
  println!("Responded with {}", response.get_code());
}
