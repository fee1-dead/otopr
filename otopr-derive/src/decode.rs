use proc_macro2::{Span, TokenStream as Ts2};

use quote::{quote, ToTokens};
use syn::DeriveInput;

use crate::common::*;

impl Field {
    pub fn match_arm(&self) -> Ts2 {
        let Field {
            const_ident,
            member,
            ty,
            ..
        } = self;
        quote! {
            #const_ident => <#ty as ::otopr::__private::Decodable>::merge_from(&mut self.#member, d)?,
        }
    }

    pub fn const_def(&self, cty: &Ts2) -> Ts2 {
        let Field {
            clean_ty,
            const_ident,
            cfg: FieldConfig { field_number, .. },
            ..
        } = self;
        quote! {
            const #const_ident: #cty = (#field_number << 3) as #cty | <<#clean_ty as ::otopr::__private::Decodable>::Wire as ::otopr::__private::WireType>::BITS;
        }
    }

    /// size this field tag takes, not considering the varint encoding.
    fn field_tag_non_varint_size(n: u64, sp: Span) -> syn::Result<usize> {
        Ok(if n < (1 << 5) {
            // 8 - 3 = 13
            // aaaaabbb - u8
            1
        } else if n < (1 << 13) {
            // 16 - 3 = 13
            // aaaaaaaa aaaaabbb - u16
            2
        } else if n < (1 << 29) {
            // 32 - 3 = 29
            4
        } else if n < (1 << 61) {
            // 64 - 3 = 61
            8
        } else {
            return Err(syn::Error::new(sp, "field number is too big!"));
        })
    }

    pub fn max_field_tag_size<'a>(it: impl IntoIterator<Item = &'a Self>) -> syn::Result<Ts2> {
        // obtain the maximum field number.
        let res = it
            .into_iter()
            .map(|t| Self::field_tag_non_varint_size(t.cfg.field_number, t.cfg.field_number_span))
            .try_fold(1, |n, r| r.map(|i| n.max(i)))?;
        //
        Ok(match res {
            1 => quote! { u8 },
            2 => quote! { u16 },
            4 => quote! { u32 },
            8 => quote! { u64 },
            _ => unreachable!(),
        })
    }

    pub fn merge(&self) -> Ts2 {
        let Field {
            member,
            ty,
            ..
        } = self;

        quote! {
            <#ty as ::otopr::__private::Decodable<'de>>::merge(&mut self.#member, other.#member);
        }
    }
}

pub(crate) fn derive_decodable_message(input: DeriveInput) -> syn::Result<Ts2> {
    let name = input.ident;
    let mut generics = input.generics;
    let impl_generics = generics.clone();
    generics.type_params_mut().for_each(|f| f.bounds.clear());
    let impl_generics = if !impl_generics.lifetimes().any(|d| d.lifetime.ident == "de") {
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
    let merges = fields.iter().map(Field::merge);

    let methods = quote! {
        type Tag = #cty;
        fn decode_field<B: ::otopr::__private::Buf>(&mut self, d: &mut ::otopr::__private::Deserializer<'de, B>, tag: Self::Tag) -> ::otopr::__private::Result<()> {
            match tag {
                #(#match_arms)*
                _ => ::otopr::__private::WireTypes::new((tag & 0b111) as u8)?.skip(d)?,
            }
            Ok(())
        }
    };

    Ok(quote! {
        #(#const_defs)*
        impl #impl_generics ::otopr::__private::DecodableMessage<'de> for #name #generics {
            #methods
        }
        impl #impl_generics ::otopr::__private::Decodable<'de> for #name #generics where Self: Default {
            type Wire = ::otopr::__private::LengthDelimitedWire;
            fn decode<B: ::otopr::__private::Buf>(d: &mut ::otopr::__private::Deserializer<'de, B>) -> ::otopr::__private::Result<Self> {
                let len = d.read_varint()?;
                let tk = d.set_limit(len);
                let message = <Self as ::otopr::__private::DecodableMessage<'de>>::decode(d);
                d.reset_limit(tk);
                Ok(message?)
            }
            fn merge(&mut self, other: Self) {
                #(#merges)*
            }
        }
    })
}
