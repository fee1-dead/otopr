use proc_macro2::{Span, TokenStream as Ts2};
use syn::{Attribute, Error, LitInt, Member, Token, Type, parenthesized, parse::{ParseStream, Parser}, parse2, spanned::Spanned};

use quote::quote;


pub struct Field {
    member: Member,
    ty: Type,
    cfg: FieldConfig,
}

impl Field {
    pub fn new(n: usize, f: syn::Field) -> syn::Result<Self> {
        let span = f
                .ident
                .as_ref()
                .map_or_else(|| f.ty.span(), |i| i.span());
        Ok(Self {
            member: f.ident.map_or_else(|| Member::from(n), Member::from),
            ty: f.ty,
            cfg: FieldConfig::from_attrs(f.attrs, span)?,
        })
    }

    pub fn encoded_size(&self) -> Ts2 {
        let Field { member, ty, cfg: FieldConfig { field_number } } = self;
        quote! {
            <#ty as ::otopr::traits::Encodable>::encoded_size(&self.#member, #field_number)
        }
    }

    pub fn encode(&self) -> syn::Result<Ts2> {
        let field_tag = self.preencoded_field_tag()?;
        let Field { member, ty, .. } = self;
        Ok(quote! {
            unsafe { 
                <#ty as ::otopr::traits::Encodable>::encode_field_precomputed(&self.#member, s, &#field_tag); 
            }
        })
    }

    pub fn preencoded_field_tag(&self) -> syn::Result<Ts2> {
        let Field { ty, cfg: FieldConfig { field_number }, .. } = self;
        Self::preencode_field_tag(field_number.base10_parse()?, ty, field_number.span())
    }

    /// given the field number and its type, return the expression that evaluates to preencoded field tag data.
    fn preencode_field_tag(n: u64, ty: &Type, sp: Span) -> syn::Result<Ts2> {
        macro_rules! err {
            ($msg: expr) => { return Err(Error::new(sp, $msg)) }
        }

        let num_bytes_it_takes = if n == 0 {
            err!("field number cannot be zero")
        } else {
            Self::field_tag_num_bytes(n)
        };

        Ok(quote! {
            ::otopr::__private::precompute_field_varint::<#ty, #num_bytes_it_takes>(#n)
        })
    }

    fn field_tag_num_bytes(n: u64) -> usize {
        if n < (1 << 4) {
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
        }
    }
}

pub struct FieldConfig {
    field_number: LitInt,
}

impl FieldConfig {
    pub fn from_attrs(attrs: Vec<Attribute>, sp: Span) -> syn::Result<Self> {
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


pub struct SynResult<T>(syn::Result<T>);

impl<C: FromIterator<T>, T> FromIterator<syn::Result<T>> for SynResult<C> {
    fn from_iter<I: IntoIterator<Item = syn::Result<T>>>(iter: I) -> Self {
        let mut err: Option<syn::Error> = None;
        let it = iter.into_iter().filter_map(|r| match r {
            Ok(t) => Some(t),
            Err(error) => {
                match &mut err {
                    Some(error2) => error2.combine(error),
                    None => err = Some(error),
                }
                None
            }
        });
        let res = C::from_iter(it);
        Self(match err {
            Some(e) => Err(e),
            None => Ok(res),
        })
    }
}

impl<T> SynResult<T> {
    pub fn inner(self) -> syn::Result<T> {
        self.0
    }
}