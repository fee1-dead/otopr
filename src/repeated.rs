use std::marker::PhantomData;
use std::ops::Deref;

use bytes::BufMut;

use crate::decoding::Decodable;
use crate::encoding::Encodable;
use crate::wire_types::WireType;
use crate::VarInt;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct Repeated<Item: ?Sized, C = Vec<Item>>(pub C, PhantomData<Item>);

impl<Item: ?Sized, C> Repeated<Item, C> {
    pub fn new(collection: C) -> Self {
        Self(collection, PhantomData)
    }
}

impl<T: ?Sized, C: Default> Default for Repeated<T, C> {
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

impl<It, T: ?Sized, C> Encodable for Repeated<T, C>
where
    It: ?Sized,
    C: Deref<Target = It>,
    for<'a> &'a It: IntoIterator<Item = &'a T>,
    T: Encodable,
{
    type Wire = T::Wire;

    fn encode(&self, _: &mut crate::encoding::ProtobufSerializer<impl BufMut>) {
        unreachable!("encode called on Repeated")
    }

    fn encode_field<V: VarInt>(
        &self,
        s: &mut crate::encoding::ProtobufSerializer<impl BufMut>,
        field_number: V,
    ) {
        let var = field_number << 3 | V::from(T::Wire::BITS);
        for t in &*self.0 {
            s.write_varint(var);
            t.encode(s);
        }
    }

    fn encoded_size<V>(&self, field_number: V) -> usize
    where
        V: VarInt,
    {
        self.0
            .into_iter()
            .map(|t| t.encoded_size(field_number))
            .sum()
    }

    unsafe fn encode_field_precomputed(
        &self,
        s: &mut crate::encoding::ProtobufSerializer<impl BufMut>,
        field_number: &[u8],
    ) {
        for t in &*self.0 {
            s.write_bytes(field_number);
            t.encode(s);
        }
    }
}

impl<'de, T: Decodable<'de>, C> Decodable<'de> for Repeated<T, C>
where
    C: Extend<T>,
    C: Default,
    C: IntoIterator<Item = T>,
{
    type Wire = T::Wire;

    fn decode<B: bytes::Buf>(
        deserializer: &mut crate::decoding::Deserializer<'de, B>,
    ) -> crate::decoding::Result<Self> {
        let mut val = Self::default();
        val.0.extend([T::decode(deserializer)?]);
        Ok(val)
    }

    fn merge(&mut self, other: Self) {
        self.0.extend(other.0)
    }

    fn merge_from<B: bytes::Buf>(
        &mut self,
        deserializer: &mut crate::decoding::Deserializer<'de, B>,
    ) -> crate::decoding::Result<()> {
        self.0.extend([T::decode(deserializer)?]);
        Ok(())
    }
}
