use std::fmt::Display;
use std::ops::BitAnd;
use std::ops::BitOr;
use std::ops::ShrAssign;
use std::str::FromStr;

use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream as Ts2;
use syn::Data;
use syn::DeriveInput;
use syn::Error;
use syn::Expr;
use syn::ExprArray;
use syn::ExprLit;
use syn::Lit;

use quote::quote;
use syn::punctuated::Punctuated;
use syn::LitInt;

use crate::common::random_ident_str;
use crate::common::SynResult;

struct Variant {
    name: Ident,
    discriminant: LitInt,
}

impl Variant {
    fn bytes_storage(&self) -> syn::Result<u8> {
        Ok(if self.discriminant.base10_parse::<u8>().is_ok() {
            1
        } else if self.discriminant.base10_parse::<u16>().is_ok() {
            2
        } else if self.discriminant.base10_parse::<u32>().is_ok() {
            4
        } else if self.discriminant.base10_parse::<u64>().is_ok() {
            8
        } else {
            return Err(Error::new_spanned(
                &self.discriminant,
                "discriminant is too big",
            ));
        })
    }
    fn varint_bytes<N>(&self) -> syn::Result<ExprArray>
    where
        N: FromStr,
        N: ToString,
        N::Err: Display,
        N: From<u8>,
        N: Ord,
        N: ShrAssign<u8>,
        N: BitAnd<Output = N>,
        N: BitOr<Output = N>,
        N: Copy,
    {
        let mut arr = ExprArray {
            attrs: vec![],
            bracket_token: syn::token::Bracket(Span::mixed_site()),
            elems: Punctuated::new(),
        };

        let mut num = self.discriminant.base10_parse::<N>()?;
        let seven_bits = N::from(0b0111_1111);
        let msb = N::from(0b1000_0000);

        while num > seven_bits {
            arr.elems.push(Expr::Lit(ExprLit {
                attrs: vec![],
                lit: Lit::Int(LitInt::new(
                    &((num & seven_bits) | msb).to_string(),
                    Span::call_site(),
                )),
            }));
            num >>= 7;
        }

        arr.elems.push(Expr::Lit(ExprLit {
            attrs: vec![],
            lit: Lit::Int(LitInt::new(&num.to_string(), Span::call_site())),
        }));

        Ok(arr)
    }
}

fn const_bytes(arr: ExprArray) -> (Ident, Ts2) {
    let name = Ident::new(&random_ident_str(), Span::mixed_site());
    let len = arr.elems.len();
    let tokens = quote! {
        const #name: [u8; #len] = #arr;
    };
    (name, tokens)
}

pub fn derive_enumeration(input: DeriveInput) -> syn::Result<Ts2> {
    let enumeration = match input.data {
        Data::Enum(e) => e,
        Data::Struct(_) => {
            return Err(Error::new_spanned(
                input,
                "cannot derive `Enumeration` on structs",
            ))
        }
        Data::Union(_) => {
            return Err(Error::new_spanned(
                input,
                "cannot derive `Enumeration` on unions",
            ))
        }
    };

    let name = input.ident;

    let mut default = None;

    let variants = enumeration
        .variants
        .into_iter()
        .map(|v| match v.fields {
            syn::Fields::Unnamed(_) | syn::Fields::Named(_) => Err(Error::new_spanned(
                v,
                "Cannot have fields on protobuf enumerations",
            )),
            syn::Fields::Unit => match v.discriminant {
                None => Err(Error::new_spanned(
                    v,
                    "must have discriminant for this variant",
                )),
                Some((_, discriminant)) => match discriminant {
                    Expr::Lit(ExprLit {
                        lit: Lit::Int(discriminant),
                        ..
                    }) => {
                        if let Ok(0u8) = discriminant.base10_parse() {
                            default = Some(v.ident.clone())
                        }
                        Ok(Variant {
                            name: v.ident,
                            discriminant,
                        })
                    }
                    disco => Err(Error::new_spanned(disco, "must be an integer literal")),
                },
            },
        })
        .collect::<SynResult<Vec<_>>>()
        .inner()?;

    let default = default.ok_or_else(|| {
        Error::new(
            Span::mixed_site(),
            "expected a default variant with the discriminant set to 0",
        )
    })?;

    let storage = variants
        .iter()
        .map(Variant::bytes_storage)
        .fold(Ok(1), |res, other| match (res, other) {
            (Ok(x), Ok(y)) => Ok(x.max(y)),
            (Ok(_), Err(e)) => Err(e),
            (Err(e), Ok(_)) => Err(e),
            (Err(mut e), Err(e2)) => {
                e.combine(e2);
                Err(e)
            }
        })?;

    let storage_ty = match storage {
        1 => quote! { u8 },
        2 => quote! { u16 },
        4 => quote! { u32 },
        8 => quote! { u64 },
        _ => unreachable!(),
    };

    let varint_bytes = variants
        .iter()
        .map(|v| match storage {
            1 => v.varint_bytes::<u8>(),
            2 => v.varint_bytes::<u16>(),
            4 => v.varint_bytes::<u32>(),
            8 => v.varint_bytes::<u64>(),
            _ => unreachable!(),
        })
        .collect::<SynResult<Vec<_>>>()
        .inner()?;

    let (variant_idents, variant_discrs): (Vec<_>, Vec<_>) = variants
        .into_iter()
        .map(|v| (v.name, v.discriminant))
        .unzip();

    let (cid, cdef): (Vec<_>, Vec<_>) = varint_bytes.into_iter().map(const_bytes).unzip();

    Ok(quote! {
        #(#cdef)*
        impl Default for #name {
            fn default() -> Self {
                Self::#default
            }
        }
        impl ::otopr::__private::Encodable for #name {
            type Wire = ::otopr::__private::VarIntWire;
            fn encoded_size<V: ::otopr::__private::VarInt>(&self, field_number: V) -> usize {
                ::otopr::VarInt::size(field_number) + match self {
                    #(Self::#variant_idents => #cid.len(),)*
                }
            }
            fn encode(&self, s: &mut ::otopr::encoding::ProtobufSerializer<impl ::otopr::__private::BufMut>) {
                s.write_bytes(match self {
                    #(Self::#variant_idents => &#cid,)*
                })
            }
        }
        impl<'a> ::otopr::__private::Decodable<'a> for #name {
            type Wire = ::otopr::__private::VarIntWire;
            fn decode<B: ::otopr::__private::Buf>(deserializer: &mut ::otopr::__private::Deserializer<'a, B>) -> ::otopr::__private::Result<Self> {
                Ok(match <#storage_ty as ::otopr::__private::VarInt>::read_field_tag(deserializer) {
                    #(Ok(#variant_discrs) => Self::#variant_idents,)*
                    Ok(_) | Err(Ok(_)) => Self::#default,
                    Err(Err(e)) => return Err(e),
                })
            }
            fn merge_from<B: ::otopr::__private::Buf>(&mut self, deserializer: &mut ::otopr::__private::Deserializer<'a, B>) -> ::otopr::__private::Result<()> {
                match <#storage_ty as ::otopr::__private::VarInt>::read_field_tag(deserializer) {
                    #(Ok(#variant_discrs) => *self = Self::#variant_idents,)*
                    Ok(_) | Err(Ok(_)) => {}
                    Err(Err(e)) => return Err(e),
                }
                Ok(())
            }
        }
    })
}
