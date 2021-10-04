use std::{borrow::Cow, str::Utf8Error};

use bytes::Buf;

use crate::{VarInt, traits::private::ArbitrarySealed, wire_types::*};

pub trait Decodable<'de>: Sized + ArbitrarySealed {
    type Wire: WireType;

    fn decode<B: Buf>(deserializer: &'de mut Deserializer<B>) -> Result<Self>;

    fn merge_from<B: Buf>(&mut self, deserializer: &'de mut Deserializer<B>) -> Result<()> {
        Ok(self.merge(Self::decode(deserializer)?))
    }

    /// If this is a `message`, call `merge()` on all fields,
    /// if this is `repeated`, extend this with the elements of `other`.
    /// for all other types simply overwrite this with `other`, which is the default.
    fn merge(&mut self, other: Self) {
        *self = other;
    }
}

pub trait DecodableMessage<'de>: Sized {
    /// How big the tag message gets. This is an unsigned varint.
    ///
    /// It is not an error if any field tag overflows this type,
    /// since there can be removed fields exceeding the current storage type.
    type Tag: VarInt;

    /// Decodes a field with the given tag. 
    ///
    /// Skips the field if there are no matches for the tag. 
    fn decode_field<D: serde::Deserializer<'de>>(&mut self, deserializer: D, tag: Self::Tag) -> Result<()>;
}

pub enum DecodingError {
    Eof,
    VarIntOverflow,
    Utf8Error(Utf8Error),
    UnknownWireType(u8),
}

impl From<Utf8Error> for DecodingError {
    fn from(e: Utf8Error) -> Self {
        Self::Utf8Error(e)
    }
}

pub type Result<T, E = DecodingError> = std::result::Result<T, E>;

pub struct Deserializer<B> {
    pub(crate) buf: B,
}

impl<B: Buf> Deserializer<B> {
    pub fn read_varint<V: VarInt>(&mut self) -> Result<V> {
        V::read(&mut self.buf)
    }

    pub fn read_bytes_borrowed<'a>(&'a mut self, len: usize) -> Result<&'a [u8]> {
        let c = self.buf.chunk();
        if c.len() >= len {
            // SAFETY: already checked above
            let c_raw = unsafe { c.get_unchecked(..len) } as *const [u8];
            self.buf.advance(len);
            Ok(unsafe { &*c_raw })
        } else {
            Err(DecodingError::Eof)
        }
    }

    pub fn read_bytes<'a>(&'a mut self, len: usize) -> Result<Cow<'a, [u8]>> {
        use bytes::BufMut;

        let c = self.buf.chunk();
        if self.buf.remaining() < len {
            Err(DecodingError::Eof)
        } else if c.len() >= len {
            // SAFETY: already checked above
            let c_raw = unsafe { c.get_unchecked(..len) } as *const [u8];
            self.buf.advance(len);
            Ok(Cow::Borrowed(unsafe { &*c_raw }))
        } else {
            let mut v = Vec::with_capacity(len);
            (&mut v).put((&mut self.buf).take(len));
            Ok(Cow::Owned(v))
        }
    }
}

impl<'de> Decodable<'de> for &'de [u8] {
    type Wire = LengthDelimitedWire;

    fn decode<B: Buf>(deserializer: &'de mut Deserializer<B>) -> Result<Self> {
        let len = deserializer.read_varint()?;
        deserializer.read_bytes_borrowed(len)
    }
}

impl<'de> Decodable<'de> for &'de str {
    type Wire = LengthDelimitedWire;

    fn decode<B: Buf>(deserializer: &'de mut Deserializer<B>) -> Result<Self> {
        let len = deserializer.read_varint()?;
        let bytes = deserializer.read_bytes_borrowed(len)?;
        Ok(std::str::from_utf8(bytes)?)
    }
}