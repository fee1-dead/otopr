use bytes::BufMut;

use crate::{
    encoding::{Encodable, EncodableMessage, ProtobufSerializer},
    wire_types::*,
    Message, VarInt,
};

pub(crate) mod private {
    pub trait Sealed {}
}

/// A type that can be encoded using ZigZag encoding.
pub trait Signable: private::Sealed {
    type Storage: Encodable;
    type From;
    fn zigzag_encode(f: Self::From) -> Self::Storage;
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
