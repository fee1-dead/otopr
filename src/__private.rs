use crate::{
    VarInt,
    encoding::ProtobufSerializer,
    traits::{Encodable},
    wire_types::WireType,
};

pub use bytes::BufMut;

pub struct __ConstBoundWorkaround<T>(T);

arbitrary_seal!(for<T> (__ConstBoundWorkaround<T>));

impl<T: Encodable> Encodable for __ConstBoundWorkaround<T> {
    type Wire = T::Wire;

    fn encoded_size<V: VarInt>(&self, _: V) -> usize {
        unreachable!()
    }

    fn encode(&self, _: &mut ProtobufSerializer<impl bytes::BufMut>) {
        unreachable!()
    }
}

/// Assumes N is the number of bytes it will take to encode a field key, returns encoded bytes in LEB128 format.
pub const unsafe fn precompute_field_varint<F, const N: usize>(mut num: u64) -> [u8; N]
where
    __ConstBoundWorkaround<F>: Encodable,
{
    let mut bytes = [0; N];
    // lowest four bits.
    let lowest_byte =
        (num << 3 & 0b0111_1000) as u8 | <__ConstBoundWorkaround<F> as Encodable>::Wire::BITS;
    num = num >> 4;
    bytes[N - 1] = lowest_byte;
    let mut n = N - 1;

    // if the length of the array to be computed is 1, then our job is done.
    if n == 0 {
        return bytes;
    }

    loop {
        // no more bits to encode.
        // this should never happen, it should panic once panicking in constants has been stabilized.
        if num == 0 {
            return bytes;
        }

        n -= 1;

        // encode the last 7 bits of `num` with the continuation bit.
        bytes[n] = (num & 0b0111_1111) as u8 | 0b1000_0000;

        // discard the last 7 bits of num.
        num = num >> 7;

        if n == 0 {
            return bytes;
        }
    }
}
