use crate::traits::{Encodable, VarInt};

pub struct ProtobufSerializer<T> {
    pub(crate) buf: T,
}

impl<T: bytes::BufMut> ProtobufSerializer<T> {
    #[inline]
    pub fn write_varint(&mut self, value: impl VarInt) {
        value.write(&mut self.buf)
    }

    fn _write_str(&mut self, s: &str) {
        self.write_varint(s.len());
        self.buf.put_slice(s.as_bytes());
    }

    #[inline]
    pub fn write_str(&mut self, s: impl AsRef<str>) {
        self._write_str(s.as_ref())
    }

    #[inline]
    pub fn encode_field<F: Encodable, V: VarInt>(&mut self, field_number: V, field: &F) {
        field.encode_field(self, field_number);
    }
}
