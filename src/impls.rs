use bytes::BufMut;

use crate::{
    decoding::{Decodable, DecodingError},
    encoding::Encodable,
    traits::Signable,
    wire_types::*,
    Fixed32, Fixed64, VarInt,
};

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

impl Encodable for bool {
    type Wire = VarIntWire;

    fn encoded_size<V: VarInt>(&self, field_number: V) -> usize {
        field_number.size() + 1
    }

    fn encode(&self, s: &mut crate::encoding::ProtobufSerializer<impl BufMut>) {
        s.write_u8(*self as u8);
    }
}

impl Decodable<'_> for bool {
    type Wire = VarIntWire;

    fn decode<B: bytes::Buf>(
        deserializer: &mut crate::decoding::Deserializer<'_, B>,
    ) -> crate::decoding::Result<Self> {
        match deserializer.get_u8() {
            0b0000_0001 => Ok(true),
            0b0000_0000 => Ok(false),
            _ => Err(DecodingError::VarIntOverflow),
        }
    }
}
