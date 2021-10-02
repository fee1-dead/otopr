use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod encode;

#[proc_macro_derive(EncodableMessage, attributes(otopr))]
pub fn derive_encodable_message(ts: TokenStream) -> TokenStream {
    encode::derive_encodable_message(parse_macro_input!(ts as DeriveInput))
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}
