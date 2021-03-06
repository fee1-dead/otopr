//! Internal module. Should only be used by macros.

pub use crate::decoding::{Decodable, DecodableMessage, Deserializer, Result};
pub use crate::encoding::{Encodable, EncodableMessage, ProtobufSerializer};
pub use crate::wire_types::*;
pub use crate::VarInt;
pub use bytes::{Buf, BufMut};

pub trait HasField<const NUM: u64> {
    type PreCompArray: AsRef<[u8]>;
    const PRECOMP: Self::PreCompArray;
}

pub trait HasFieldDecode<const NUM: u64> {
    type VarInt: VarInt;
    const FNUM: Self::VarInt;
}

pub struct __ConstBoundWorkaround<T>(T);

impl<T: WireType> WireType for __ConstBoundWorkaround<T> {
    const BITS: u8 = T::BITS;
}

impl<T: WireType> crate::traits::private::Sealed for __ConstBoundWorkaround<T> {}

/// Assumes N is the number of bytes it will take to encode a field key, returns encoded bytes in LEB128 format.
///
/// # Safety
/// You must ensure that `N` is the number of bytes that will be encoded.
pub const unsafe fn precompute_field_varint<F, const N: usize>(mut num: u64) -> [u8; N]
where
    __ConstBoundWorkaround<F>: WireType,
{
    let mut bytes = [0; N];
    // lowest four bits.
    let lowest_byte =
        (num << 3 & 0b0111_1000) as u8 | <__ConstBoundWorkaround<F> as WireType>::BITS;
    num >>= 4;
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
        num >>= 7;

        if n == 0 {
            return bytes;
        }
    }
}
