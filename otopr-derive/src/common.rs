use std::time::SystemTime;

use proc_macro2::{Ident, Span, TokenStream as Ts2};
use syn::{Attribute, Data, DeriveInput, Error, LitInt, Member, Token, Type, parenthesized, parse::{ParseStream, Parser}, parse2, spanned::Spanned};

use quote::quote;

pub fn fields_from(input: Data) -> syn::Result<Vec<Field>> {
    let fields = match input {
        Data::Struct(ds) => ds.fields,
        Data::Enum(_) => {
            return Err(Error::new(
                Span::call_site(),
                "enumerations are not yet supported",
            ))
        }
        Data::Union(_) => return Err(Error::new(Span::call_site(), "unions are not supported")),
    };

    fields
        .into_iter()
        .enumerate()
        .map(|(n, field)| Field::new(n, field))
        .collect::<SynResult<_>>().inner()
}

pub struct Field {
    member: Member,
    ty: Type,
    cfg: FieldConfig,
    const_ident: Ident,
}

pub fn random_ident_str() -> String {
    format!("_OTOPR_DERIVE_INTERNAL_{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs())
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
            const_ident: Ident::new(&random_ident_str(), Span::call_site()),
        })
    }

    pub fn encoded_size(&self) -> Ts2 {
        let Field { member, ty, cfg: FieldConfig { field_number, .. }, .. } = self;
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

    pub fn match_arm(&self) -> Ts2 {
        let Field { const_ident, member, ty, .. } = self;
        quote! {
            #const_ident => <#ty as ::otopr::decoding::Decodable>::merge_from(&mut self.#member, d)?,
        }
    }

    pub fn const_def(&self, cty: &Ts2) -> Ts2 {
        let Field { ty, const_ident, cfg: FieldConfig { field_number, .. }, .. } = self;
        quote! {
            const #const_ident: #cty = (#field_number << 3) as #cty | <<#ty as ::otopr::decoding::Decodable>::Wire as ::otopr::wire_types::WireType>::BITS;
        }
    }

    pub fn preencoded_field_tag(&self) -> syn::Result<Ts2> {
        let Field { ty, cfg: FieldConfig { field_number, field_number_span }, .. } = self;
        Self::preencode_field_tag(*field_number, ty, *field_number_span)
    }

    /// given the field number and its type, return the expression that evaluates to preencoded field tag data.
    fn preencode_field_tag(n: u64, ty: &Type, sp: Span) -> syn::Result<Ts2> {
        macro_rules! err {
            ($msg: expr) => { return Err(Error::new(sp, $msg)) }
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
        } else if n < (1 << 61) { // 61 bits field number, 3 bits wire type
            10
        } else {
            return Err(syn::Error::new(sp, "field number is too big!"))
        })
    }

    /// size this field tag takes, not considering the varint encoding.
    fn field_tag_non_varint_size(n: u64, sp: Span) -> syn::Result<usize> {
        Ok(if n < (1 << 5) { // 8 - 3 = 13
            // aaaaabbb - u8
            1
        } else if n < (1 << 13) { // 16 - 3 = 13
            // aaaaaaaa aaaaabbb - u16
            2
        } else if n < (1 << 29) { // 32 - 3 = 29
            4
        } else if n < (1 << 61) { // 64 - 3 = 61
            8
        } else {
            return Err(syn::Error::new(sp, "field number is too big!"))
        })
    }

    pub fn max_field_tag_size<'a>(it: impl IntoIterator<Item = &'a Self>) -> syn::Result<Ts2> {
        // obtain the maximum field number.
        let res = it
            .into_iter()
            .map(|t| Self::field_tag_non_varint_size(t.cfg.field_number, t.cfg.field_number_span))
            .try_fold(0, |n, r| r.map(|i| n.max(i)))?;
        // 
        Ok(match res {
            1 => quote! { u8 },
            2 => quote! { u16 },
            4 => quote! { u32 },
            8 => quote! { u64 },
            _ => unreachable!(),
        })
    }
}

pub struct FieldConfig {
    field_number: u64,
    field_number_span: Span,
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
        let mut field_number_span = Span::call_site();
        for attr in attrs.into_iter().filter(|a| a.path.is_ident("otopr")) {
            let (_parens, tts) = Parser::parse2(|p: ParseStream| {
                let content;
                Ok((
                    parenthesized!(content in p),
                    content.parse_terminated::<Ts2, Token![,]>(|p| p.parse())?,
                ))
            }, attr.tokens)?;
            if tts.len() == 1 {
                let l: LitInt = parse2(tts.first().unwrap().clone())?;
                field_number = Some(l.base10_parse()?);
                field_number_span = l.span();
            } else {
                try_opt!(None, "expected one argument");
            }
        }

        let field_number = try_opt!(field_number, "missing field number for field");
        Ok(FieldConfig { field_number, field_number_span })
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