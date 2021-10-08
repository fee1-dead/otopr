use std::iter::FromIterator;
use std::time::SystemTime;

use proc_macro2::{Ident, Span, TokenStream as Ts2};
use syn::{Attribute, Data, Error, GenericArgument, LitInt, Member, PathArguments, Token, Type, TypeArray, TypeGroup, TypeParen, TypePath, TypePtr, TypeReference, TypeSlice, TypeTuple, parenthesized, parse::{ParseStream, Parser}, parse2, spanned::Spanned};

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
        .collect::<SynResult<_>>()
        .inner()
}

pub struct Field {
    pub member: Member,
    pub ty: Type,
    pub clean_ty: Type,
    pub cfg: FieldConfig,
    pub const_ident: Ident,
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
            | Type::Reference(TypeReference { elem, .. })
            | Type::Slice(TypeSlice { elem, .. })
            | Type::Paren(TypeParen { elem, .. }) => clean_ty_inner(elem),
            Type::Tuple(TypeTuple { elems, .. }) => elems.iter_mut().for_each(clean_ty_inner),
            Type::Path(TypePath { path, .. }) => {
                for segment in &mut path.segments {
                    if let PathArguments::AngleBracketed(args) = &mut segment.arguments {
                        for arg in &mut args.args {
                            match arg {
                                GenericArgument::Lifetime(lt) => lt.ident = Ident::new("_", lt.ident.span()),
                                GenericArgument::Type(ty) => clean_ty_inner(ty),
                                _ => {}
                            }
                        }
                    }
                }
            },
            _ => unimplemented!(),
        }
    }
    clean_ty_inner(&mut ty);
    ty
}

impl Field {
    pub fn new(n: usize, f: syn::Field) -> syn::Result<Self> {
        let span = f.ident.as_ref().map_or_else(|| f.ty.span(), |i| i.span());
        Ok(Self {
            member: f.ident.map_or_else(|| Member::from(n), Member::from),
            ty: f.ty.clone(),
            clean_ty: clean_ty(f.ty),
            cfg: FieldConfig::from_attrs(f.attrs, span)?,
            const_ident: Ident::new(&random_ident_str(), Span::call_site()),
        })
    }
}

pub struct FieldConfig {
    pub field_number: u64,
    pub field_number_span: Span,
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
            let (_parens, tts) = Parser::parse2(
                |p: ParseStream| {
                    let content;
                    let parens = parenthesized!(content in p);
                    Ok((
                        parens,
                        content.parse_terminated::<Ts2, Token![,]>(|p| p.parse())?,
                    ))
                },
                attr.tokens,
            )?;
            if tts.len() == 1 {
                let l: LitInt = parse2(tts.first().unwrap().clone())?;
                field_number = Some(l.base10_parse()?);
                field_number_span = l.span();
            } else {
                try_opt!(None, "expected one argument");
            }
        }

        let field_number = try_opt!(field_number, "missing field number for field");
        Ok(FieldConfig {
            field_number,
            field_number_span,
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
