use bytes::BufMut;

use crate::{Fixed32, Fixed64, VarInt, traits::{Encodable, Signable, private::ArbitrarySealed}, wire_types::*};



macro_rules! signable {
    ($($id:ident($storage: ty)),*) => {$(
        impl Signable for $id {
            type Storage = $storage;
            type From = Self;
            fn zigzag_encode(this: Self) -> $storage {
                const BITS_M1: u32 = <$id>::BITS - 1;
                ((this << 1) ^ (this >> BITS_M1)) as $storage
            }
        }
    )*};
    ($($id:ident($storage: ty) = $signed:ident),*) => {$(
        impl Signable for $id {
            type Storage = $storage;
            type From = $signed;
            fn zigzag_encode(this: $signed) -> $storage {
                const BITS_M1: u32 = <$signed>::BITS - 1;
                ((this << 1) ^ (this >> BITS_M1)) as $storage
            }
        }
    )*};
}

signable!(i32(u32), i64(u64));
signable!(Fixed32(u32) = i32, Fixed64(u32) = i64);

crate::seal! {
    for u64,
    for u32,
    for i64,
    for i32,
    for u16,
    for u8,
    for usize,
    for Fixed32,
    for Fixed64,
}

impl Encodable for str {
    type Wire = LengthDelimitedWire;

    fn encoded_size<V: VarInt>(&self, field_number: V) -> usize {
        field_number.size() + self.len().size() + self.len()
    }
    fn encode(&self, s: &mut crate::encoding::ProtobufSerializer<impl BufMut>) {
        s.write_str(self)
    }
}

arbitrary_seal!((str));

impl<'a, T: ArbitrarySealed + ?Sized> ArbitrarySealed for &'a T {}
impl<'a, T: ArbitrarySealed + ?Sized> ArbitrarySealed for &'a mut T {}