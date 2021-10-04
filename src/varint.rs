use std::ops::{BitOr, Shl};

use crate::decoding::*;
use crate::traits::*;
use crate::wire_types::*;

use bytes::{Buf, BufMut};

/// A VarInt type.
pub trait VarInt:
    private::Sealed + Copy + Shl<usize, Output = Self> + From<u8> + BitOr<Output = Self> + 'static
{
    fn write(self, buf: &mut impl bytes::BufMut);
    fn read(buf: &mut impl bytes::Buf) -> crate::decoding::Result<Self>;
    fn read_field_tag(buf: &mut impl bytes::Buf) -> Result<Self, crate::decoding::Result<WireTypes>>;
    fn size(self) -> usize;
}

#[cold]
#[inline]
fn eof<T>() -> Result<T> {
    Err(DecodingError::Eof)
}

#[cold]
#[inline]
fn overflow<T>() -> Result<T> {
    Err(DecodingError::VarIntOverflow)
}

macro_rules! varint {
    (common($intty:ident)) => {
        arbitrary_seal!(($intty));
        impl Encodable for $intty {
            type Wire = VarIntWire;

            fn encoded_size<V: VarInt>(&self, field_number: V) -> usize {
                self.size() + field_number.size()
            }
            fn encode(&self, s: &mut crate::encoding::ProtobufSerializer<impl BufMut>) {
                s.write_varint(*self)
            }
        }
        impl Decodable<'_> for $intty {
            type Wire = VarIntWire;
            fn decode<B: Buf>(s: &mut Deserializer<B>) -> Result<$intty> {
                s.read_varint()
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

                fn read_field_tag(buf: &mut impl bytes::Buf) -> Result<Self, Result<WireTypes>> {
                    if !buf.has_remaining() {
                        return Err(eof());
                    }

                    let mut storage = 0;

                    let mut byte = buf.get_u8();
                    let mut shift = 0;
                    while byte > 0b0111_1111 {
                        storage |= ((byte & 0b0111_1111) as $intty) << shift;
                        shift += 7;

                        macro_rules! overflow {
                            () => {{
                                while byte > 0b0111_1111 {
                                    byte = buf.get_u8();
                                }
                                return Err(WireTypes::new(byte & 0b111))
                            }}
                        }

                        if shift > <$intty>::BITS {
                            // overflow
                            overflow!()
                        }

                        let bits_left = <$intty>::BITS - shift;
                        if bits_left < 8 {
                            // more bits than we can fit
                            if (8 - byte.leading_zeros()) > bits_left {
                                // overflow
                                overflow!()
                            }
                        }

                        if !buf.has_remaining() {
                            return Err(eof());
                        }

                        byte = buf.get_u8();
                    }

                    storage |= (byte as $intty) << shift;

                    Ok(storage)
                }

                fn read(buf: &mut impl bytes::Buf) -> Result<$intty> {
                    if !buf.has_remaining() {
                        return eof();
                    }

                    let mut storage = 0;

                    let mut byte = buf.get_u8();
                    let mut shift = 0;
                    while byte > 0b0111_1111 {
                        storage |= ((byte & 0b0111_1111) as $intty) << shift;
                        shift += 7;

                        if shift > <$intty>::BITS {
                            // overflow
                            return overflow()
                        }

                        let bits_left = <$intty>::BITS - shift;
                        if bits_left < 8 {
                            // more bits than we can fit
                            if (8 - byte.leading_zeros()) > bits_left {
                                // overflow
                                return overflow();
                            }
                        }

                        if !buf.has_remaining() {
                            return eof();
                        }

                        byte = buf.get_u8();
                    }

                    storage |= (byte as $intty) << shift;

                    Ok(storage)
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
            fn read(buf: &mut impl bytes::Buf) -> Result<$selfty> {
                <$otherty as VarInt>::read(buf).map(|n| n as $selfty)
            }
            fn read_field_tag(buf: &mut impl bytes::Buf) -> Result<$selfty, Result<WireTypes>> {
                <$otherty as VarInt>::read_field_tag(buf).map(|n| n as $selfty)
            }
            #[inline]
            fn size(self) -> usize {
                VarInt::size(self as $otherty)
            }
        }
        varint!(common($selfty));
    )*};
}

varint!(u64, u32, u16, usize);
varint_forward!(i32 as u32, i64 as u64);