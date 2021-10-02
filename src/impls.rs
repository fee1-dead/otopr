use bytes::BufMut;

use crate::{
    traits::{Encodable, Signable, VarInt},
    wire_types::*,
    Fixed32, Fixed64, Repeated,
};

macro_rules! varint {
    (common($intty:ident)) => {
        arbitrary_seal!(for $intty);
        impl Encodable for $intty {
            type Wire = VarIntWire;

            fn encoded_size<V: VarInt>(&self, field_number: V) -> usize {
                self.size() + field_number.size()
            }
            fn encode(&self, s: &mut crate::encoding::ProtobufSerializer<impl BufMut>) {
                s.write_varint(*self)
            }
        }
    };
    ($($intty:ident),*) => {
        $(
            impl VarInt for $intty {
                fn write(mut self, buf: &mut impl bytes::BufMut) {
                    while self > 0b1000_0000 {
                        // truncate to the last eight bits and set the
                        // most significant bit to 1.
                        buf.put_u8(self as u8 | 0b1000_0000);
                        self >>= 7;
                    }
                    buf.put_u8(self as u8);
                }
                fn size(self) -> usize {
                    const BITS_M1: u32 = <$intty>::BITS - 1;

                    fn log2_floor_nonzero(n: $intty) -> u32 {
                        BITS_M1 ^ n.leading_zeros()
                    }

                    // https://github.com/protocolbuffers/protobuf/blob/3.3.x/src/google/protobuf/io/coded_stream.h#L1301-L1309

                    let log2_value = log2_floor_nonzero(self | 0x1);

                    ((log2_value * 9 + 73) / 64) as usize
                }
            }
            varint!(common($intty));
        )*
    };
}

macro_rules! varint_forward {
    ($($selfty:ident as $otherty:ty),*) => {$(
        impl VarInt for $selfty {
            #[inline]
            fn write(self, buf: &mut impl bytes::BufMut) {
                VarInt::write(self as $otherty, buf)
            }
            #[inline]
            fn size(self) -> usize {
                VarInt::size(self as $otherty)
            }
        }
        varint!(common($selfty));
    )*};
}

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
    for usize,
    for Fixed32,
    for Fixed64,
}

varint!(u64, u32, u16, usize);
varint_forward!(i32 as u32, i64 as u64);

impl Encodable for str {
    type Wire = LengthDelimitedWire;

    fn encoded_size<V: VarInt>(&self, field_number: V) -> usize {
        field_number.size() + self.len().size() + self.len()
    }
    fn encode(&self, s: &mut crate::encoding::ProtobufSerializer<impl BufMut>) {
        s.write_str(self)
    }
}

impl<T, C> Encodable for Repeated<C>
where
    for<'a> &'a C: IntoIterator<Item = &'a T>,
    T: Encodable,
{
    type Wire = T::Wire;

    fn encode(&self, _: &mut crate::encoding::ProtobufSerializer<impl BufMut>) {
        unreachable!("encode called on Repeated")
    }

    fn encode_field<V: VarInt>(
        &self,
        s: &mut crate::encoding::ProtobufSerializer<impl BufMut>,
        field_number: V,
    ) {
        let var = field_number << 3 | V::from(T::Wire::BITS);
        for t in &self.0 {
            s.write_varint(var);
            t.encode(s);
        }
    }

    fn encoded_size<V>(&self, field_number: V) -> usize
    where
        V: VarInt,
    {
        self.0
            .into_iter()
            .map(|t| t.encoded_size(field_number))
            .sum()
    }
}

arbitrary_seal!(for<T> Repeated<T>, for str,);
