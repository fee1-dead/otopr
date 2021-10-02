use proc_macro2::{Span, TokenStream as Ts2};

use quote::quote;
use syn::{Attribute, Data, DeriveInput, Error, LitInt, Member, Token, Type, parenthesized, parse::{ParseStream, Parser}, parse2, spanned::Spanned};

struct Field {
    member: Member,
    ty: Type,
    cfg: FieldConfig,
}

struct FieldConfig {
    field_number: LitInt,
}

impl FieldConfig {
    fn from_attrs(attrs: Vec<Attribute>, sp: Span) -> syn::Result<Self> {
        macro_rules! try_opt {
            ($e:expr, $msg: literal) => {
                match $e {
                    Some(t) => t,
                    None => return Err(Error::new(sp, $msg)),
                }
            };
        }

        let mut field_number = None;
        for attr in attrs.into_iter().filter(|a| a.path.is_ident("otopr")) {
            let (_parens, tts) = Parser::parse2(|p: ParseStream| {
                let content;
                Ok((
                    parenthesized!(content in p),
                    content.parse_terminated::<Ts2, Token![,]>(|p| p.parse())?,
                ))
            }, attr.tokens)?;
            if tts.len() == 1 {
                field_number = Some(parse2(tts.first().unwrap().clone())?);
            } else {
                try_opt!(None, "expected one argument");
            }
        }

        let field_number = try_opt!(field_number, "missing field number for field");
        Ok(FieldConfig { field_number })
    }
}

pub(crate) fn derive_encodable_message(input: DeriveInput) -> syn::Result<Ts2> {
    let name = input.ident;

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
        .map(|(n, field)| {
            let span = field
                .ident
                .as_ref()
                .map_or_else(|| field.ty.span(), |i| i.span());

            syn::Result::Ok(Field {
                member: field
                    .ident
                    .clone()
                    .map_or_else(|| Member::from(n), Member::from),
                ty: field.ty,
                cfg: FieldConfig::from_attrs(field.attrs, span)?,
            })
        })
        .collect::<Result<_, _>>()?;

    let field_members: Vec<_> = fields.iter().map(|f| &f.member).collect();
    let field_tys: Vec<_> = fields.iter().map(|f| &f.ty).collect();
    let field_numbers = fields.iter().map(|f| &f.cfg.field_number);
    let field_tags = fields
        .iter()
        .map(|f| &f.cfg.field_number)
        .map(|i| (i.span(), i.base10_parse::<u64>()))
        .zip(field_tys.clone())
        .map(|((sp, n), ty)| {
            preencode_field_tag(n?, ty, sp)
        }).collect::<Result<Vec<_>, _>>()?;

    let methods = quote! {
        fn encoded_size(&self, ) -> usize {
            0 #(+ <#field_tys as ::otopr::traits::Encodable>::encoded_size(&self.#field_members, #field_numbers))*
        }
        fn encode<T: ::otopr::__private::BufMut>(&self, s: &mut ::otopr::encoding::ProtobufSerializer<T>) {
            #(unsafe { 
                <#field_tys as ::otopr::traits::Encodable>::encode_field_precomputed(&self.#field_members, s, &#field_tags); 
            })*
        }
    };


    Ok(quote! {
        impl ::otopr::traits::EncodableMessage for #name {
            #methods
        }
    })
}

/// given the field number and its type, return the expression that evaluates to preencoded field tag data.
fn preencode_field_tag(n: u64, ty: &Type, sp: Span) -> syn::Result<Ts2> {
    macro_rules! err {
        ($msg: expr) => { return Err(Error::new(sp, $msg)) }
    }
    let num_bytes_it_takes: usize = if n == 0 {
        err!("field number cannot be zero")
    } else if n < (1 << 4) {
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
    } else {
        10
    };

    Ok(quote! {
        ::otopr::__private::precompute_field_varint::<#ty, #num_bytes_it_takes>(#n)
    })
}
