use std::ops::{BitOr, Shl};

use bytes::BufMut;

use crate::{Message, encoding::ProtobufSerializer, wire_types::*};

pub(crate) mod private {
    pub trait Sealed {}
    pub trait ArbitrarySealed {}
}

/// A type that can be encoded using ZigZag encoding.
pub trait Signable: private::Sealed {
    type Storage: Encodable;
    type From;
    fn zigzag_encode(f: Self::From) -> Self::Storage;
}

/// A VarInt type.
pub trait VarInt:
    private::Sealed + Copy + Shl<usize, Output = Self> + From<u8> + BitOr<Output = Self>
{
    fn write(self, buf: &mut impl bytes::BufMut);
    fn size(self) -> usize;
}

pub trait Encodable: private::ArbitrarySealed {
    type Wire: WireType;

    /// returns the size in bytes when encoded, including the field number.
    fn encoded_size<V: VarInt>(&self, field_number: V) -> usize;
    fn encode(&self, s: &mut ProtobufSerializer<impl BufMut>);

    /// The entry point to encoding `Encodable`s in a message.
    ///
    /// the default implementation writes field_number << 3 | wire_type as an varint and calls [`encode()`].
    fn encode_field<V: VarInt>(&self, s: &mut ProtobufSerializer<impl BufMut>, field_number: V) {
        let var = field_number << 3 | V::from(Self::Wire::BITS);
        s.write_varint(var);
        self.encode(s);
    }

    unsafe fn encode_field_precomputed(
        &self,
        s: &mut ProtobufSerializer<impl BufMut>,
        field_number: &[u8],
    ) {
        s.buf.put_slice(field_number);
        self.encode(s);
    }
}

pub trait EncodableMessage {
    fn encoded_size(&self) -> usize;
    fn encode<T: BufMut>(&self, s: &mut ProtobufSerializer<T>);
}

impl<T: EncodableMessage> Encodable for Message<T> {
    type Wire = LengthDelimitedWire;

    fn encoded_size<V: VarInt>(&self, field_number: V) -> usize {
        let calc_size = EncodableMessage::encoded_size(&self.0);

        // encode field number, the size as varint, plus the bytes that follow.
        field_number.size() + calc_size.size() + calc_size
    }

    fn encode(&self, s: &mut ProtobufSerializer<impl BufMut>) {
        s.write_varint(EncodableMessage::encoded_size(&self.0));
        EncodableMessage::encode(&self.0, s)
    }
}

arbitrary_seal!(for<T> Message<T>);
