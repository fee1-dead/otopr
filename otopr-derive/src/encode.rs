use proc_macro2::{Span, TokenStream as Ts2};

use quote::quote;
use syn::{DeriveInput, Error};

use crate::common::*;

impl Field {
    pub fn encoded_size(&self) -> Ts2 {
        let Field {
            member,
            ty,
            cfg: FieldConfig { field_number, encode_via, .. },
            ..
        } = self;


        if let Some((_, expr)) = encode_via {
            quote! {{
                use ::otopr::__private::Encodable;
                let x = &self.#member;
                let encode = #expr;
                encode.encoded_size(#field_number)
            }}
        } else {
            quote! {{
                <#ty as ::otopr::__private::Encodable>::encoded_size(&self.#member, #field_number)
            }}
        }
    }

    pub fn encode(&self) -> syn::Result<Ts2> {
        let Field { member, ty, cfg: FieldConfig { encode_via, .. }, const_ident_encode, .. } = self;
        let tt = if let Some((_, expr)) = encode_via {
            quote! {
                {
                    use ::otopr::__private::Encodable;
                    let x = &self.#member;
                    let encode = #expr;
                    unsafe {
                        encode.encode_field_precomputed(s, &#const_ident_encode);
                    }
                }
            }
        } else {
            quote! {
                unsafe {
                    <#ty as ::otopr::__private::Encodable>::encode_field_precomputed(&self.#member, s, &#const_ident_encode);
                }
            }
        };
        Ok(tt)
    }

    pub fn wire_ty(&self) -> Ts2 {
        let self_ty = &self.clean_ty;
        match &self.cfg.encode_via {
            Some((ty, _)) => quote! { #ty },
            None => quote! { <#self_ty as ::otopr::__private::Encodable>::Wire },
        }
    }

    pub fn const_def_encode(&self) -> syn::Result<Ts2> {
        let Field {
            cfg:
                FieldConfig {
                    field_number,
                    field_number_span,
                    ..
                },
            const_ident_encode,
            ..
        } = self;
        let ty = self.wire_ty();
        let num_bytes_it_takes = if *field_number == 0 {
            return Err(Error::new(*field_number_span, "field number cannot be zero"));
        } else {
            Self::field_tag_num_bytes(*field_number, *field_number_span)?
        };
        
        Ok(quote! {
            const #const_ident_encode: [u8; #num_bytes_it_takes]
                = unsafe {
                    ::otopr::__private::precompute_field_varint::<#ty, #num_bytes_it_takes>(#field_number)
                };
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
    let where_clause = impl_generics.where_clause;
    let impl_generics = {
        let params = impl_generics.params;
        let lt = impl_generics.lt_token;
        let gt = impl_generics.gt_token;
        quote! {
            #lt #params #gt
        }
    };

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
        fn encode<__BufMut: ::otopr::__private::BufMut>(&self, s: &mut ::otopr::__private::ProtobufSerializer<__BufMut>) {
            #(#field_encodes)*
        }
    };

    let const_defs = fields.iter().map(Field::const_def_encode).collect::<SynResult<Vec<_>>>().inner()?;

    Ok(quote! {
        #(#const_defs)*
    
        impl #impl_generics ::otopr::__private::EncodableMessage for #name #generics #where_clause {
            #methods
        }
        impl #impl_generics ::otopr::__private::Encodable for #name #generics #where_clause {
            type Wire = ::otopr::__private::LengthDelimitedWire;

            fn encoded_size<__VarInt: ::otopr::__private::VarInt>(&self, field_number: __VarInt) -> usize {
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
