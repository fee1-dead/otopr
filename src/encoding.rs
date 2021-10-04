use bytes::BufMut;

use crate::{Fixed32, Fixed64, Signed, VarInt, traits::{Encodable, Signable}, wire_types::*};

pub struct ProtobufSerializer<T> {
    pub(crate) buf: T,
}

impl<T: bytes::BufMut> ProtobufSerializer<T> {
    #[inline]
    pub fn new(buf: T) -> Self {
        Self {
            buf
        }
    }

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
    pub fn write_u32(&mut self, n: u32) {
        self.buf.put_u32_le(n)
    }

    #[inline]
    pub fn write_u64(&mut self, n: u64) {
        self.buf.put_u64_le(n)
    }

    #[inline]
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.buf.put_slice(bytes)
    }

    #[inline]
    pub fn encode_field<F: Encodable, V: VarInt>(&mut self, field_number: V, field: &F) {
        field.encode_field(self, field_number);
    }
}

arbitrary_seal!((Fixed32), (Fixed64), for<T: Signable> (Signed<T>), ([u8]));

impl Encodable for Fixed32 {
    type Wire = Fixed32Wire;

    fn encoded_size<V: VarInt>(&self, field_number: V) -> usize {
        field_number.size() + 4
    }

    fn encode(&self, s: &mut ProtobufSerializer<impl BufMut>) {
        s.write_u32(self.0)
    }
}

impl Encodable for Fixed64 {
    type Wire = Fixed64Wire;

    fn encoded_size<V: VarInt>(&self, field_number: V) -> usize {
        field_number.size() + 8
    }

    fn encode(&self, s: &mut ProtobufSerializer<impl BufMut>) {
        s.write_u64(self.0)
    }
}

impl<T: Signable> Encodable for Signed<T> {
    type Wire = <T::Storage as Encodable>::Wire;

    fn encoded_size<V: VarInt>(&self, field_number: V) -> usize {
        self.0.encoded_size(field_number)
    }

    fn encode(&self, s: &mut ProtobufSerializer<impl BufMut>) {
        self.0.encode(s)
    }
}

impl Encodable for [u8] {
    type Wire = LengthDelimitedWire;

    fn encoded_size<V: VarInt>(&self, field_number: V) -> usize {
        field_number.size() + self.len().size() + self.len()
    }

    fn encode(&self, s: &mut ProtobufSerializer<impl BufMut>) {
        s.write_varint(self.len());
        s.write_bytes(self)
    }
}

impl<'a, T: Encodable + ?Sized> Encodable for &'a T {
    type Wire = T::Wire;

    fn encoded_size<V: VarInt>(&self, field_number: V) -> usize {
        T::encoded_size(*self, field_number)
    }

    fn encode(&self, s: &mut crate::encoding::ProtobufSerializer<impl BufMut>) {
        T::encode(*self, s)
    }

    fn encode_field<V: VarInt>(&self, s: &mut ProtobufSerializer<impl BufMut>, field_number: V) {
        (*self).encode_field(s, field_number)
    }

    unsafe fn encode_field_precomputed(
        &self,
        s: &mut ProtobufSerializer<impl BufMut>,
        field_number: &[u8],
    ) {
        (*self).encode_field_precomputed(s, field_number)
    }
}