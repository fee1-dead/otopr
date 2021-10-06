use proc_macro2::TokenStream as Ts2;

use quote::{ToTokens, quote};
use syn::DeriveInput;

use crate::common::*;

pub(crate) fn derive_decodable_message(input: DeriveInput) -> syn::Result<Ts2> {
    let name = input.ident;
    let mut generics = input.generics;
    let impl_generics = generics.clone();
    generics.type_params_mut().for_each(|f| f.bounds.clear());
    let impl_generics = if !impl_generics.lifetimes().any(|d| d.lifetime.ident == "de" ) {
        let params = impl_generics.params;
        quote! {
            <'de, #params>
        }
    } else {
        impl_generics.into_token_stream()
    };

    let fields = fields_from(input.data)?;
    
    let max = Field::max_field_tag_size(&fields)?;
    let cty = &max;

    let const_defs = fields.iter().map(|f| f.const_def(cty));
    let match_arms = fields.iter().map(Field::match_arm);

    let methods = quote! {
        type Tag = #cty;
        fn decode_field<B: ::otopr::__private::Buf>(&mut self, d: &mut ::otopr::decoding::Deserializer<'de, B>, tag: Self::Tag) -> ::otopr::decoding::Result<()> {
            match tag {
                #(#match_arms)*
                _ => ::otopr::wire_types::WireTypes::new((tag & 0b111) as u8)?.skip(d)?,
            }
            Ok(())
        }
    };

    Ok(quote! {
        #(#const_defs)*
        impl #impl_generics ::otopr::decoding::DecodableMessage<'de> for #name #generics {
            #methods
        }
    })
}
