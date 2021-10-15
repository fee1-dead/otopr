use proc_macro2::Ident;
use syn::{Attribute, Error, Token, TypeParam, WhereClause, parenthesized, parse::{ParseStream, Parser}, punctuated::Punctuated};
use syn::Ident as SynIdent;

pub struct InputCfg {
    pub encode_where_clause: Option<WhereClause>,
    pub encode_extra_type_params: Option<Punctuated<TypeParam, Token![,]>>,
}

impl InputCfg {
    pub fn from_attrs(attrs: Vec<Attribute>) -> syn::Result<Self> {
        let mut encode_where_clause = None;
        let mut encode_extra_type_params = None;
        for attr in attrs {
            if attr.path.get_ident().map(|id| id == "otopr").unwrap_or_default() {
                Parser::parse2(|ps: ParseStream| {
                    let content;
                    let _ = parenthesized!(content in ps);
                    if content.peek(SynIdent) {
                        let id: Ident = content.parse()?;
                        if id == "encode_where_clause" {
                            let content2;
                            let _ = parenthesized!(content2 in content);
                            encode_where_clause = Some(content2.parse()?);
                        } else if id == "encode_extra_type_params" {
                            let content2;
                            let _ = parenthesized!(content2 in content);
                            encode_extra_type_params = Some(Punctuated::parse_terminated(&content2)?);
                        } else {
                            return Err(Error::new_spanned(id, "expected `encode_where_clause` or `encode_extra_type_params`"))
                        }
                    } else {
                        return Err(Error::new(content.span(), "expected identifier"))
                    }
                    Ok(())
                }, attr.tokens)?;
            }
        }

        Ok(Self {
            encode_where_clause,
            encode_extra_type_params,
        })
    }
}