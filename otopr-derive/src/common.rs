use std::iter::FromIterator;
use std::time::SystemTime;

use proc_macro2::{Ident, Span, TokenStream as Ts2};
use syn::{
    parenthesized,
    parse::{Parse, ParseStream, Parser},
    parse2, Attribute, Data, Error, Expr, GenericArgument, LitInt, Member, PathArguments, Token,
    Type, TypeArray, TypeGroup, TypeParen, TypePath, TypePtr, TypeReference, TypeSlice, TypeTuple,
};

mod input_cfg;
pub use input_cfg::*;

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

    let mut default_field_number = 1;

    fields
        .into_iter()
        .enumerate()
        .map(|(n, field)| {
            let res = Field::new(n, field, default_field_number);
            if let Ok(f) = &res {
                default_field_number = f.cfg.field_number + 1;
            }
            res
        })
        .collect::<SynResult<_>>()
        .inner()
}

pub struct Field {
    pub member: Member,
    pub ty: Type,
    pub clean_ty: Type,
    pub cfg: FieldConfig,
    pub const_ident_decode: Ident,
    pub const_ident_encode: Ident,
}

pub fn random_ident_str() -> String {
    format!(
        "_OTOPR_DERIVE_INTERNAL_{}",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

fn clean_ty(mut ty: Type) -> Type {
    fn clean_ty_inner(ty: &mut Type) {
        match ty {
            Type::Array(TypeArray { elem, .. })
            | Type::Group(TypeGroup { elem, .. })
            | Type::Ptr(TypePtr { elem, .. })
            | Type::Slice(TypeSlice { elem, .. })
            | Type::Paren(TypeParen { elem, .. }) => clean_ty_inner(elem),
            Type::Reference(TypeReference { elem, lifetime, .. }) => {
                if let Some(lt) = lifetime {
                    lt.ident = Ident::new("_", lt.ident.span());
                }
                clean_ty_inner(elem)
            }
            Type::Tuple(TypeTuple { elems, .. }) => elems.iter_mut().for_each(clean_ty_inner),
            Type::Path(TypePath { path, .. }) => {
                for segment in &mut path.segments {
                    if let PathArguments::AngleBracketed(args) = &mut segment.arguments {
                        for arg in &mut args.args {
                            match arg {
                                GenericArgument::Lifetime(lt) => {
                                    lt.ident = Ident::new("_", lt.ident.span())
                                }
                                GenericArgument::Type(ty) => clean_ty_inner(ty),
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ => unimplemented!(),
        }
    }
    clean_ty_inner(&mut ty);
    ty
}

impl Field {
    pub fn new(n: usize, f: syn::Field, default_field_number: u64) -> syn::Result<Self> {
        Ok(Self {
            member: f.ident.map_or_else(|| Member::from(n), Member::from),
            ty: f.ty.clone(),
            clean_ty: clean_ty(f.ty),
            cfg: FieldConfig::from_attrs(f.attrs, default_field_number)?,
            const_ident_decode: Ident::new(&random_ident_str(), Span::call_site()),
            const_ident_encode: Ident::new(&random_ident_str(), Span::call_site()),
        })
    }
}

pub struct FieldConfig {
    pub field_number: u64,
    pub field_number_span: Span,
    pub encode_via: Option<(Type, Expr)>,
}

impl FieldConfig {
    pub fn from_attrs(attrs: Vec<Attribute>, default_field_number: u64) -> syn::Result<Self> {
        let mut field_number = None;
        let mut encode_via = None;
        let mut field_number_span = Span::call_site();
        for attr in attrs.into_iter().filter(|a| a.path.is_ident("otopr")) {
            let OtoprAttr {
                field_number: f,
                encode_via: ev,
            } = parse2(attr.tokens)?;
            if let Some(f) = f {
                field_number = Some(f.base10_parse()?);
                field_number_span = f.span();
            }
            if let Some(ev) = ev {
                encode_via = Some(ev);
            }
        }

        let field_number = field_number.unwrap_or(default_field_number);
        Ok(FieldConfig {
            field_number,
            field_number_span,
            encode_via,
        })
    }
}

pub struct OtoprAttr {
    pub field_number: Option<LitInt>,
    pub encode_via: Option<(Type, Expr)>,
}

impl Parse for OtoprAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let _ = parenthesized!(content in input);
        let tts = content.parse_terminated::<Ts2, Token![,]>(|p| p.parse())?;
        let mut field_number = None;
        let mut via = None;
        for ts in tts {
            Parser::parse2(
                |p: ParseStream| {
                    let lookahead = p.lookahead1();
                    if lookahead.peek(LitInt) {
                        field_number = Some(p.parse()?);
                    } else if lookahead.peek(syn::Ident) {
                        let id: Ident = p.parse()?;
                        if id == "encode_via" {
                            let content;
                            let _ = parenthesized!(content in p);
                            let ty = content.parse()?;
                            let _: Token![,] = content.parse()?;
                            let expr = content.parse()?;
                            via = Some((ty, expr));
                        } else {
                            return Err(Error::new_spanned(id, "expected 'encode_via'"));
                        }
                    } else {
                        return Err(lookahead.error());
                    }
                    Ok(())
                },
                ts,
            )?;
        }
        Ok(Self {
            field_number,
            encode_via: via,
        })
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
