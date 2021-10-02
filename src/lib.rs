//! otoPr - Obvious Rust Protobuf Library
//!
//! This library focuses on API design and performance.

pub(crate) extern crate self as otopr;

pub use otopr_derive::EncodableMessage;

#[macro_use]
mod macros {
    #[macro_export]
    macro_rules! arbitrary_seal {
        ($(for$(<$($id:ident$(: $bound:path)?),+ $(,)?>)? $ty:path),+ $(,)?) => {
            $(
                impl$(<$($id$(: $bound)?),*>)? crate::traits::private::ArbitrarySealed for $ty {}
            )*
        };
    }
    #[macro_export]
    macro_rules! seal {
        ($(for$(<$($id:ident$(: $bound:path)?),+ $(,)?>)? $ty:path),+ $(,)?) => {
            $(
                impl$(<$($id$(: $bound)?),*>)? crate::traits::private::Sealed for $ty {}
            )*
        };
    }
}

pub mod traits;

pub mod encoding;
pub mod ser;

mod impls;
pub mod wire_types;

pub mod __private;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Packed<T>(T);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Repeated<T>(T);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Signed<T: traits::Signable>(T);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Fixed32(u32);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct Fixed64(u64);

#[allow(non_camel_case_types)]
pub mod types {
    use crate::*;

    pub type sfixed64 = Signed<Fixed32>;
    pub type sfixed32 = Signed<Fixed64>;
}
