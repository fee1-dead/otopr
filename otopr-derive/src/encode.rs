use proc_macro2::{Span, TokenStream as Ts2};

use quote::quote;
use syn::{DeriveInput, Error, Type};

use crate::common::*;

impl Field {
    pub fn encoded_size(&self) -> Ts2 {
        let Field {
            member,
            ty,
            cfg: FieldConfig { field_number, encode_via, .. },
            ..
        } = self;


        if let Some((ty, expr)) = encode_via {
            quote! {{
                let x = &self.#member;
                let encode: #ty = #expr;
                <#ty as ::otopr::__private::Encodable>::encoded_size(&encode, #field_number)
            }}
        } else {
            quote! {{
                <#ty as ::otopr::__private::Encodable>::encoded_size(&self.#member, #field_number)
            }}
        }
    }

    pub fn encode(&self) -> syn::Result<Ts2> {
        let field_tag = self.preencoded_field_tag()?;
        let Field { member, ty, cfg: FieldConfig { encode_via, .. }, .. } = self;
        let tt = if let Some((newty, expr)) = encode_via {
            quote! {
                {
                    let x = &self.#member;
                    let encode: #newty = #expr;
                    unsafe {
                        <#newty as ::otopr::__private::Encodable>::encode_field_precomputed(&encode, s, &#field_tag);
                    }
                }
            }
        } else {
            quote! {
                unsafe {
                    <#ty as ::otopr::__private::Encodable>::encode_field_precomputed(&self.#member, s, &#field_tag);
                }
            }
        };
        Ok(tt)
    }

    pub fn ty(&self) -> &Type {
        match &self.cfg.encode_via {
            Some((ty, _)) => ty,
            None => &self.ty,
        }
    }

    pub fn preencoded_field_tag(&self) -> syn::Result<Ts2> {
        let Field {
            cfg:
                FieldConfig {
                    field_number,
                    field_number_span,
                    ..
                },
            ..
        } = self;
        let ty = self.ty();
        Self::preencode_field_tag(*field_number, ty, *field_number_span)
    }

    /// given the field number and its type, return the expression that evaluates to preencoded field tag data.
    fn preencode_field_tag(n: u64, ty: &Type, sp: Span) -> syn::Result<Ts2> {
        macro_rules! err {
            ($msg: expr) => {
                return Err(Error::new(sp, $msg))
            };
        }

        let num_bytes_it_takes = if n == 0 {
            err!("field number cannot be zero")
        } else {
            Self::field_tag_num_bytes(n, sp)?
        };

        Ok(quote! {
            ::otopr::__private::precompute_field_varint::<#ty, #num_bytes_it_takes>(#n)
        })
    }

    fn field_tag_num_bytes(n: u64, sp: Span) -> syn::Result<usize> {
        Ok(if n < (1 << 4) {
            // 0aaaabbb where bbb is wire type.
            1
        } else if n < (1 << 11) {
            // 0aaaaaaa 0aaaabbb, +7 bits available
            2
        } else if n < (1 << 18) {
            // +7 bits
            3
        } else if n < (1 << 25) {
            // ...
            4
        } else if n < (1 << 32) {
            5
        } else if n < (1 << 39) {
            6
        } else if n < (1 << 46) {
            7
        } else if n < (1 << 53) {
            8
        } else if n < (1 << 60) {
            9
        } else if n < (1 << 61) {
            // 61 bits field number, 3 bits wire type
            10
        } else {
            return Err(syn::Error::new(sp, "field number is too big!"));
        })
    }
}

pub(crate) fn derive_encodable_message(input: DeriveInput) -> syn::Result<Ts2> {
    let name = input.ident;
    let impl_generics = input.generics;
    let mut generics = impl_generics.clone();
    generics.type_params_mut().for_each(|f| f.bounds.clear());

    let fields = fields_from(input.data)?;

    let field_encoded_sizes = fields.iter().map(Field::encoded_size);
    let field_encodes: Vec<_> = fields
        .iter()
        .map(Field::encode)
        .collect::<SynResult<_>>()
        .inner()?;

    let methods = quote! {
        fn encoded_size(&self) -> usize {
            0 #(+ #field_encoded_sizes)*
        }
        fn encode<T: ::otopr::__private::BufMut>(&self, s: &mut ::otopr::__private::ProtobufSerializer<T>) {
            #(#field_encodes)*
        }
    };

    Ok(quote! {
        impl #impl_generics ::otopr::__private::EncodableMessage for #name #generics {
            #methods
        }
        impl #impl_generics ::otopr::__private::Encodable for #name #generics {
            type Wire = ::otopr::__private::LengthDelimitedWire;

            fn encoded_size<V: ::otopr::__private::VarInt>(&self, field_number: V) -> usize {
                let calc_size = ::otopr::__private::EncodableMessage::encoded_size(self);

                // encode field number, the size as varint, plus the bytes that follow.
                ::otopr::__private::VarInt::size(field_number) + ::otopr::__private::VarInt::size(calc_size) + calc_size
            }

            fn encode(&self, s: &mut ::otopr::__private::ProtobufSerializer<impl ::otopr::__private::BufMut>) {
                s.write_varint(::otopr::__private::EncodableMessage::encoded_size(self));
                ::otopr::__private::EncodableMessage::encode(self, s)
            }
        }
    })
}
