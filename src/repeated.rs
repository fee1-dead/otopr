use std::ops::Deref;

use bytes::BufMut;

use crate::decoding::Decodable;
use crate::encoding::{Encodable, ProtobufSerializer};
use crate::wire_types::WireType;
use crate::VarInt;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
#[repr(transparent)]
pub struct Repeated<C>(pub C);

impl<C> Repeated<C> {
    pub fn new(collection: C) -> Self {
        Self(collection)
    }
}

impl<'a, C> From<&'a C> for &'a Repeated<C> {
    fn from(c: &'a C) -> Self {
        let ptr = c as *const C as *const Repeated<C>;
        // SAFETY: Repeated is #[repr(transparent)] over C so this dereference is safe.
        unsafe { &*ptr }
    }
}

impl<C> Repeated<C>
where
    C: Deref,
{
    pub fn map<'a, NewIter, F: Fn(<&'a C::Target as IntoIterator>::IntoIter) -> NewIter>(
        &'a self,
        f: F,
    ) -> RepeatedMap<<&'a C::Target as IntoIterator>::IntoIter, F>
    where
        &'a C::Target: IntoIterator,
    {
        RepeatedMap {
            collection: self.0.into_iter(),
            map: f,
        }
    }
}

pub struct RepeatedMap<Iter, F> {
    collection: Iter,
    map: F,
}

impl<'a, F, IntoIt, NewIt> RepeatedMap<IntoIt, F>
where
    F: Fn(IntoIt) -> NewIt,
    IntoIt: Clone,
    NewIt: Iterator,
{
    fn mk_encoder(&self) -> RepeatedEncoder<NewIt> {
        RepeatedEncoder((self.map)(self.collection.clone()))
    }
}

impl<C> Repeated<C>
where
    C: Deref,
{
    fn mk_encoder<'a>(
        &'a self,
    ) -> RepeatedEncoder<<&'a C::Target as IntoIterator>::IntoIter>
    where
        &'a C::Target: IntoIterator,
        <&'a C::Target as IntoIterator>::Item: Encodable,
    {
        RepeatedEncoder(self.0.into_iter())
    }
}

macro_rules! mk_encoder_trait_impls {
    () => {
        fn encode(&self, _: &mut crate::encoding::ProtobufSerializer<impl BufMut>) {
            unreachable!("encode called on Repeated")
        }

        fn encode_field<V: VarInt>(
            &self,
            s: &mut crate::encoding::ProtobufSerializer<impl BufMut>,
            field_number: V,
        ) {
            self.mk_encoder().encode_field(s, field_number)
        }

        fn encoded_size<V>(&self, field_number: V) -> usize
        where
            V: VarInt,
        {
            self.mk_encoder().encoded_size(field_number)
        }

        unsafe fn encode_field_precomputed(
            &self,
            s: &mut crate::encoding::ProtobufSerializer<impl BufMut>,
            field_number: &[u8],
        ) {
            self.mk_encoder().encode_field_precomputed(s, field_number)
        }
    };
}

impl<C> Encodable for Repeated<C>
where
    C: Deref,
    C::Target: IntoIterator,
    for<'a> &'a C::Target: IntoIterator<Item = &'a <<C as Deref>::Target as IntoIterator>::Item>,
    <<C as Deref>::Target as IntoIterator>::Item: Encodable,
{
    type Wire = <<C::Target as IntoIterator>::Item as Encodable>::Wire;

    mk_encoder_trait_impls!();
}

impl<F, IntoIt, NewIt, Item> Encodable for RepeatedMap<IntoIt, F>
where
    F: Fn(IntoIt) -> NewIt,
    NewIt: Iterator<Item = Item>,
    Item: ?Sized + Encodable,
    IntoIt: Clone,
{
    type Wire = Item::Wire;

    mk_encoder_trait_impls!();
}

struct RepeatedEncoder<Iter: Iterator>(Iter);

impl<Iter> RepeatedEncoder<Iter>
where
    Iter: Iterator,
    Iter::Item: Encodable,
{
    pub fn encode_field<V: VarInt>(self, s: &mut ProtobufSerializer<impl BufMut>, field_number: V) {
        let var = field_number << 3 | V::from(<<Iter as Iterator>::Item as Encodable>::Wire::BITS);
        for t in self.0 {
            s.write_varint(var);
            t.encode(s);
        }
    }

    pub fn encoded_size<V: VarInt>(self, field_number: V) -> usize {
        self.0.map(|t| t.encoded_size(field_number)).sum()
    }

    pub unsafe fn encode_field_precomputed(
        self,
        s: &mut crate::encoding::ProtobufSerializer<impl BufMut>,
        field_number: &[u8],
    ) {
        for t in self.0 {
            s.write_bytes(field_number);
            t.encode(s);
        }
    }
}

impl<'de, C> Decodable<'de> for Repeated<C>
where
    C: Extend<C::Item>,
    C: Default,
    C: IntoIterator,
    C::Item: Decodable<'de>,
{
    type Wire = <<C as IntoIterator>::Item as Decodable<'de>>::Wire;

    fn decode<B: bytes::Buf>(
        deserializer: &mut crate::decoding::Deserializer<'de, B>,
    ) -> crate::decoding::Result<Self> {
        let mut val = Self::default();
        val.0.extend([C::Item::decode(deserializer)?]);
        Ok(val)
    }

    fn merge(&mut self, other: Self) {
        self.0.extend(other.0)
    }

    fn merge_from<B: bytes::Buf>(
        &mut self,
        deserializer: &mut crate::decoding::Deserializer<'de, B>,
    ) -> crate::decoding::Result<()> {
        self.0.extend([C::Item::decode(deserializer)?]);
        Ok(())
    }
}

mod test {
    // use std::ops::Deref;

    use otopr::*;

    /// Generic struct that holds any sequences of bytes.
    #[derive(otopr::EncodableMessage)]
    #[otopr(encode_extra_type_params(TItem))]
    #[otopr(encode_where_clause(
        where
            for<'a> &'a T: IntoIterator<Item = &'a TItem>,
            TItem: AsRef<[u8]>,
            for<'a> <&'a T as IntoIterator>::IntoIter: Clone,
    ))]
    pub struct Testing<T> {
        #[otopr(encode_via(wire_types::LengthDelimitedWire, <&Repeated<&T>>::from(&x).map(|it| it.map(AsRef::as_ref))))]
        foo: T,
    }

    /// Assert that the types are well-formed, that is, all predicates on the type's `Encodable` impl are fulfilled.
    macro_rules! assert_wf {
        ($($ty:ty),+$(,)?) => {
            #[allow(unreachable_code)]
            fn __assert_wf() {
                $(
                    <$ty as otopr::__private::Encodable>::encoded_size(todo!(), 0);
                )+
            }
        };
    }

    assert_wf! {
        Testing<Vec<Vec<u8>>>,
        //Testing<&'static [&'static str]>,
        //Testing<&[[u8; 10]; 10]>,
        Testing<[[u8; 10]; 10]>,
        // ^ does not support yet
    }
}
