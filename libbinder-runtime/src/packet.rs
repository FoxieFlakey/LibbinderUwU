use std::marker::PhantomData;

pub struct Packet<'runtime> {
  runtime: PhantomData<&'runtime ()>
}

