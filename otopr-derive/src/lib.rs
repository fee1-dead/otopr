#![deny(warnings)]
#![deny(unused_lifetimes)]
// No unsafe code should ever be used in derive macros.
#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod common;
mod decode;
mod encode;
mod enumeration;

#[proc_macro_derive(EncodableMessage, attributes(otopr))]
pub fn derive_encodable_message(ts: TokenStream) -> TokenStream {
    encode::derive_encodable_message(parse_macro_input!(ts as DeriveInput))
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}

#[proc_macro_derive(DecodableMessage, attributes(otopr))]
pub fn derive_decodable_message(ts: TokenStream) -> TokenStream {
    decode::derive_decodable_message(parse_macro_input!(ts as DeriveInput))
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}

#[proc_macro_derive(Enumeration)]
pub fn derive_enumeration(ts: TokenStream) -> TokenStream {
    enumeration::derive_enumeration(parse_macro_input!(ts as DeriveInput))
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}
