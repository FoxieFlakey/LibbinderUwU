const TYPE_LARGE: u8 = 0x85;

const fn pack_chars(c1: u8, c2: u8, c3: u8, c4: u8) -> u32 {
  ((c1 as u32) << 24) |
  ((c2 as u32) << 16) |
  ((c3 as u32) << 8) |
  (c4 as u32)
}

pub const BINDER: u32 = pack_chars(b's', b'b', b'*', TYPE_LARGE);
#[expect(unused)]
pub const WEAK_BINDER: u32 = pack_chars(b'w', b'b', b'*', TYPE_LARGE);
pub const HANDLE: u32 = pack_chars(b's', b'h', b'*', TYPE_LARGE);
#[expect(unused)]
pub const WEAK_HANDLE: u32 = pack_chars(b'w', b'h', b'*', TYPE_LARGE);
#[expect(unused)]
pub const FD: u32 = pack_chars(b'f', b'd', b'*', TYPE_LARGE);
#[expect(unused)]
pub const FDA: u32 = pack_chars(b'f', b'd', b'a', TYPE_LARGE);
#[expect(unused)]
pub const PTR: u32 = pack_chars(b'p', b't', b'*', TYPE_LARGE);
