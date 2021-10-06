use proc_macro2::TokenStream as Ts2;

use quote::quote;
use syn::DeriveInput;

use crate::common::*;

pub(crate) fn derive_encodable_message(input: DeriveInput) -> syn::Result<Ts2> {
    let name = input.ident;
    let mut generics = input.generics;
    generics.type_params_mut().for_each(|f| f.bounds.clear());

    let fields = fields_from(input.data)?;

    let field_encoded_sizes = fields.iter().map(Field::encoded_size);
    let field_encodes: Vec<_> = fields.iter().map(Field::encode).collect::<SynResult<_>>().inner()?;

    let methods = quote! {
        fn encoded_size(&self) -> usize {
            0 #(+ #field_encoded_sizes)*
        }
        fn encode<T: ::otopr::__private::BufMut>(&self, s: &mut ::otopr::encoding::ProtobufSerializer<T>) {
            #(#field_encodes)*
        }
    };


    Ok(quote! {
        impl #generics ::otopr::traits::EncodableMessage for #name #generics {
            #methods
        }
    })
}

