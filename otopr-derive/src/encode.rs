use proc_macro2::{Span, TokenStream as Ts2};

use quote::quote;
use syn::{Data, DeriveInput, Error, Member, spanned::Spanned};

use crate::common::*;


pub(crate) fn derive_encodable_message(input: DeriveInput) -> syn::Result<Ts2> {
    let name = input.ident;
    let generics = input.generics;

    let fields = match input.data {
        Data::Struct(ds) => ds.fields,
        Data::Enum(_) => {
            return Err(Error::new(
                Span::call_site(),
                "enumerations are not yet supported",
            ))
        }
        Data::Union(_) => return Err(Error::new(Span::call_site(), "unions are not supported")),
    };

    let fields: Vec<Field> = fields
        .into_iter()
        .enumerate()
        .map(|(n, field)| Field::new(n, field))
        .collect::<SynResult<_>>().inner()?;

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

