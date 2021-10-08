use bytes::Buf;

use crate::{
    decoding::{DecodingError, Deserializer, Result},
    traits::private::Sealed,
};

/// A WireType.
pub trait WireType: Sealed {
    const BITS: u8;
}

macro_rules! wires {
    ($($id:ident = $bits:expr),*$(,)?) => {
        $(
            seal!(for $id);
            pub struct $id;
            impl WireType for $id {
                const BITS: u8 = $bits;
            }
        )*
        #[repr(u8)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub enum WireTypes {
            $($id = $bits),*
        }
        impl WireTypes {
            pub fn new(raw: u8) -> Result<Self> {
                match raw {
                    $($bits => Ok(Self::$id),)*
                    _ => Err(DecodingError::UnknownWireType(raw)),
                }
            }
        }
    };
}

wires! {
    VarIntWire = 0,
    Fixed64Wire = 1,
    LengthDelimitedWire = 2,
    // unsupported wires
    // StartGroupWire = 3,
    // EndGroupWire = 4,
    Fixed32Wire = 5,
}

impl WireTypes {
    pub fn skip<B: Buf>(self, d: &mut Deserializer<B>) -> Result<()> {
        match self {
            WireTypes::VarIntWire => while d.buf.get_u8() > 0b0111_1111 {},
            WireTypes::Fixed64Wire => d.buf.advance(8),
            WireTypes::LengthDelimitedWire => {
                let len = d.read_varint()?;
                d.buf.advance(len);
            }
            WireTypes::Fixed32Wire => d.buf.advance(4),
        }
        Ok(())
    }
}
