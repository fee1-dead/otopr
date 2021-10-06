use std::{borrow::Cow, str::Utf8Error};

use bytes::Buf;

use crate::{Message, VarInt, traits::private::ArbitrarySealed, wire_types::*};

pub trait Decodable<'de>: Sized + ArbitrarySealed {
    type Wire: WireType;

    fn decode<B: Buf>(deserializer: &mut Deserializer<'de, B>) -> Result<Self>;

    fn merge_from<B: Buf>(&mut self, deserializer: &mut Deserializer<'de, B>) -> Result<()> {
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
    fn decode_field<B: Buf>(&mut self, deserializer: &mut Deserializer<'de, B>, tag: Self::Tag) -> Result<()>;
    fn decode<B: Buf>(deserializer: &mut Deserializer<'de, B>) -> Result<Self> where Self: Default {
        let mut message = Self::default();
        loop {
            if !deserializer.has_remaining() {
                break;
            }
            match Self::Tag::read_field_tag(deserializer) {
                Ok(tag) => message.decode_field(deserializer, tag)?,
                Err(Ok(wire)) => wire.skip(deserializer)?,
                Err(Err(e)) => return Err(e),
            }
        }
        Ok(message)
    }
}

#[derive(Debug)]
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

pub struct LimitToken {
    prev_limit: usize,
    set_to: usize,
}

impl Drop for LimitToken {
    fn drop(&mut self) {
        // panic!("Don't forget to reset the limit!!!!!!")
    }
}

pub struct Deserializer<'de, B> {
    pub(crate) buf: &'de mut B,
    limit: usize
}

impl<'de, B: Buf> Deserializer<'de, B> {
    pub fn new(buf: &'de mut B) -> Self {
        Self {
            buf,
            limit: usize::MAX,
        }
    }

    pub fn set_limit(&mut self, limit: usize) -> LimitToken {
        let prev_limit = self.limit;
        let set_to = limit.min(self.limit);
        self.limit = set_to;

        LimitToken {
            prev_limit,
            set_to,
        }
    }

    pub fn reset_limit(&mut self, token: LimitToken) {
        let limit_used = token.set_to - self.limit;
        self.limit = token.prev_limit - limit_used;

        // avoid panicking.
        std::mem::forget(token);
    }

    /// get an u8 from the underlying buffer, assuming this is within limits.
    pub fn get_u8(&mut self) -> u8 {
        if self.limit != usize::MAX { self.limit -= 1 }
        self.buf.get_u8()
    }

    pub fn has_remaining(&self) -> bool {
        self.buf.remaining() != 0 && self.limit != 0
    }

    pub fn check_limit<'a, F: FnOnce(&'a mut B) -> Result<V>, V>(&'a mut self, len: usize, f: F) -> Result<V> {
        if self.limit == usize::MAX {
            f(&mut self.buf)
        } else if self.limit < len {
            Err(DecodingError::Eof)
        } else {
            self.limit -= len;
            f(&mut self.buf)
        }
    }

    pub fn read_varint<V: VarInt>(&mut self) -> Result<V> {
        V::read(self)
    }

    pub fn read_bytes_borrowed<'a>(&mut self, len: usize) -> Result<&'a [u8]> {
        self.check_limit(len, |buf| {
            let c = buf.chunk();
            if c.len() >= len {
                // SAFETY: already checked above
                let c_raw = unsafe { c.get_unchecked(..len) } as *const [u8];
                buf.advance(len);
                Ok(unsafe { &*c_raw })
            } else {
                Err(DecodingError::Eof)
            }
        })
    }

    pub fn read_bytes<'a>(&mut self, len: usize) -> Result<Cow<'a, [u8]>> {
        use bytes::BufMut;
        self.check_limit(len, |buf| {
            let c = buf.chunk();
            if buf.remaining() < len {
                Err(DecodingError::Eof)
            } else if c.len() >= len {
                // SAFETY: already checked above
                let c_raw = unsafe { c.get_unchecked(..len) } as *const [u8];
                buf.advance(len);
                Ok(Cow::Borrowed(unsafe { &*c_raw }))
            } else {
                let mut v = Vec::with_capacity(len);
                (&mut v).put(buf.take(len));
                Ok(Cow::Owned(v))
            }
        })
    }

    pub fn read_bytes_owned(&mut self, len: usize) -> Result<Vec<u8>> {
        use bytes::BufMut;
        self.check_limit(len, |buf| {
            if buf.remaining() < len {
                Err(DecodingError::Eof)
            } else {
                let mut v = Vec::with_capacity(len);
                (&mut v).put(buf.take(len));
                Ok(v)
            }
        })
    }
}

impl<'de> Decodable<'de> for &'de [u8] {
    type Wire = LengthDelimitedWire;

    fn decode<B: Buf>(deserializer: &mut Deserializer<'de, B>) -> Result<Self> {
        let len = deserializer.read_varint()?;
        deserializer.read_bytes_borrowed(len)
    }
}

impl Decodable<'_> for Vec<u8> {
    type Wire = LengthDelimitedWire;

    fn decode<B: Buf>(deserializer: &mut Deserializer<'_, B>) -> Result<Self> {
        let len = deserializer.read_varint()?;
        deserializer.read_bytes_owned(len)
    }
}

impl<'de> Decodable<'de> for &'de str {
    type Wire = LengthDelimitedWire;

    fn decode<B: Buf>(deserializer: &mut Deserializer<'de, B>) -> Result<Self> {
        let len = deserializer.read_varint()?;
        let bytes = deserializer.read_bytes_borrowed(len)?;
        Ok(std::str::from_utf8(bytes)?)
    }
}

impl Decodable<'_> for String {
    type Wire = LengthDelimitedWire;

    fn decode<B: Buf>(deserializer: &mut Deserializer<'_, B>) -> Result<Self> {
        let len = deserializer.read_varint()?;
        let bytes = deserializer.read_bytes_owned(len)?;
        Ok(String::from_utf8(bytes).map_err(|e| e.utf8_error())?)
    }
}

arbitrary_seal!((String), (Vec<u8>));

impl<'de, M: DecodableMessage<'de> + Default> Decodable<'de> for Message<M> {
    type Wire = LengthDelimitedWire;

    fn decode<B: Buf>(deserializer: &mut Deserializer<'de, B>) -> Result<Self> {
        let len = deserializer.read_varint()?;
        let tk = deserializer.set_limit(len);
        let message = M::decode(deserializer);
        deserializer.reset_limit(tk);
        Ok(Message(message?))
    }
}