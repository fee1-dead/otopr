use std::{collections::HashMap, marker::PhantomData};

use bytes::Buf;

use crate::decoding::{Decodable, DecodingError, Deserializer};
use crate::encoding::Encodable;
use crate::{wire_types::*, VarInt};

pub struct Map<K, V, T = HashMap<K, V>>(T, PhantomData<(K, V)>);

impl<K, V, T> Encodable for Map<K, V, T>
where
    for<'a> &'a T: IntoIterator<Item = (&'a K, &'a V)>,
    K: Encodable,
    V: Encodable,
{
    type Wire = LengthDelimitedWire;

    fn encoded_size<I: crate::VarInt>(&self, field_number: I) -> usize {
        (&self.0)
            .into_iter()
            .map(|(key, value)| {
                let pair_message_size = key.encoded_size(1) + value.encoded_size(2);
                field_number.size() + pair_message_size.size() + pair_message_size
            })
            .sum::<usize>()
    }

    fn encode_field<I: VarInt>(
        &self,
        s: &mut crate::encoding::ProtobufSerializer<impl bytes::BufMut>,
        field_number: I,
    ) {
        let var = field_number << 3 | I::from(Self::Wire::BITS);
        for (key, value) in &self.0 {
            s.write_varint(var);
            s.write_varint(key.encoded_size(1) + value.encoded_size(2));
            unsafe {
                key.encode_field_precomputed(s, &[1]);
                value.encode_field_precomputed(s, &[2]);
            }
        }
    }

    unsafe fn encode_field_precomputed(
        &self,
        s: &mut crate::encoding::ProtobufSerializer<impl bytes::BufMut>,
        field_number: &[u8],
    ) {
        for (key, value) in &self.0 {
            s.write_bytes(field_number);
            s.write_varint(key.encoded_size(1) + value.encoded_size(2));
            key.encode_field_precomputed(s, &[0b00001_000 | K::Wire::BITS]);
            value.encode_field_precomputed(s, &[0b00010_000 | V::Wire::BITS]);
        }
    }

    fn encode(&self, _s: &mut crate::encoding::ProtobufSerializer<impl bytes::BufMut>) {
        unreachable!("encode called for Map")
    }
}

impl<'de, K, V, T> Decodable<'de> for Map<K, V, T>
where
    K: Decodable<'de> + Default,
    V: Decodable<'de> + Default,
    T: Extend<(K, V)> + Default,
    T: IntoIterator<Item = (K, V)>,
{
    type Wire = LengthDelimitedWire;

    fn merge_from<B: Buf>(&mut self, d: &mut Deserializer<'de, B>) -> crate::decoding::Result<()> {
        let k_fn = 0b00001_000 | K::Wire::BITS;
        let v_fn = 0b00010_000 | V::Wire::BITS;

        let msg_len = d.read_varint()?;
        let lmt = d.set_limit(msg_len);
        let mut key = None;
        let mut value = None;
        let mut error = None;
        for _ in 0..2 {
            if d.has_remaining() {
                match u8::read_field_tag(d) {
                    Ok(n) if n == k_fn => key = Some(K::decode(d)),
                    Ok(n) if n == v_fn => value = Some(V::decode(d)),
                    Ok(n) => error = WireTypes::new(n).and_then(|w| w.skip(d)).err(),
                    Err(Ok(w)) => error = w.skip(d).err(),
                    Err(Err(e)) => error = Some(e),
                }
            }
        }
        while d.has_remaining() {
            match u8::read_field_tag(d) {
                Ok(n) => error = WireTypes::new(n).and_then(|w| w.skip(d)).err(),
                Err(Ok(w)) => error = w.skip(d).err(),
                Err(Err(e)) => error = Some(e),
            }
        }
        d.reset_limit(lmt);

        if let Some(e) = error {
            return Err(e);
        }

        let key = match key {
            Some(res) => res?,
            None => K::default(),
        };

        let value = match value {
            Some(res) => res?,
            None => V::default(),
        };

        self.0.extend([(key, value)]);

        Ok(())
    }

    fn merge(&mut self, other: Self) {
        self.0.extend(other.0)
    }

    fn decode<B>(d: &mut Deserializer<'de, B>) -> std::result::Result<Self, DecodingError>
    where
        B: Buf,
    {
        let mut this = Self(T::default(), PhantomData);
        this.merge_from(d)?;
        Ok(this)
    }
}
