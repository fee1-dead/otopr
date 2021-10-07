//! otoPr - Obvious Rust Protobuf Library
//!
//! This library focuses on API design and performance.

pub(crate) extern crate self as otopr;

pub use otopr_derive::*;

#[macro_use]
mod macros {
    #[macro_export]
    macro_rules! arbitrary_seal {
        ($($(for<$($id:ident$(: $bound:path)?),+ $(,)?>)? ($ty:ty)),+ $(,)?) => {
            $(
                impl$(<$($id$(: $bound)?),*>)? crate::traits::private::ArbitrarySealed for $ty {}
            )*
        };
    }
    #[macro_export]
    macro_rules! seal {
        ($(for$(<$($id:ident$(: $bound:path)?),+ $(,)?>)? $ty:ty),+ $(,)?) => {
            $(
                impl$(<$($id$(: $bound)?),*>)? crate::traits::private::Sealed for $ty {}
            )*
        };
    }
}

pub mod prelude;

pub mod traits;

pub mod encoding;
pub mod decoding;

mod repeated;
pub use repeated::Repeated;

mod varint;
pub use varint::VarInt;

mod map;

#[cfg(test)]
pub mod tests;

mod impls;
pub mod wire_types;

pub mod __private;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
#[repr(transparent)]
pub struct Packed<T>(T);



#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
#[repr(transparent)]
pub struct Signed<T: traits::Signable>(T::Storage);

impl<T: traits::Signable> Signed<T> {
    /// Creates a new instance of `Signed` by encoding using zigzag.
    pub fn new(t: T::From) -> Self {
        Self(T::zigzag_encode(t))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
#[repr(transparent)]
pub struct Fixed32(u32);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
#[repr(transparent)]
pub struct Fixed64(u64);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
#[repr(transparent)]
pub struct Message<T>(T);

impl<T> Message<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

#[allow(non_camel_case_types)]
pub mod types {
    use crate::*;

    pub type sfixed64 = Signed<Fixed32>;
    pub type sfixed32 = Signed<Fixed64>;
}
