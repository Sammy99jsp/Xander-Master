use proc_macro2::Span;
use syn::{parse::Parse, spanned::Spanned};

pub enum NamespaceTy {
    New {
        id: syn::LitStr,
        _at: syn::Token![@],
        ident: syn::Ident,
    },
    Existing {
        _at: syn::Token![@],
        path: syn::Path,
    },
}

impl Parse for NamespaceTy {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        match input.peek(syn::LitStr) {
            true => Ok(Self::New {
                id: input.parse()?,
                _at: input.parse()?,
                ident: input.parse()?,
            }),
            false => Ok(Self::Existing {
                _at: input.parse()?,
                path: input.parse()?,
            }),
        }
    }
}

pub enum ArchiveTrait {
    New(Option<syn::token::Paren>, Option<syn::Ident>),
    Existing {
        paren: syn::token::Paren,
        _at: syn::Token![@],
        path: syn::Path,
    },
}

impl ArchiveTrait {
    pub fn span(&self) -> Option<Span> {
        match self {
            ArchiveTrait::New(paren, ..) => paren.map(|p| p.span.span()),
            ArchiveTrait::Existing { paren: _paren, .. } => Some(_paren.span.span()),
        }
    }
}

impl Parse for ArchiveTrait {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if !input.peek(syn::token::Paren) {
            return Ok(Self::New(None, None));
        }

        let inner;
        let paren = syn::parenthesized!(inner in input);

        match inner.peek(syn::Token![@]) {
            true => Ok(Self::Existing {
                paren,
                _at: inner.parse()?,
                path: inner.parse()?,
            }),
            false => Ok(Self::New(
                Some(paren),
                if inner.is_empty() {
                    None
                } else {
                    inner.parse()?
                },
            )),
        }
    }
}

#[derive(Debug)]
pub struct Serialize(pub syn::Ident);

#[derive(Debug)]
pub struct Deserialize(pub syn::Ident);

#[derive(Debug)]
pub struct CheckBytes(pub syn::Ident);

#[derive(Debug)]
pub struct Singleton(pub syn::Ident);

pub struct NamespaceDeriveArgs {
    pub archive_tr: Option<(syn::Ident, ArchiveTrait)>,
    pub serialize: Option<Serialize>,
    pub deserialize: Option<Deserialize>,
    pub check_bytes: Option<CheckBytes>,
    pub singleton: Option<Singleton>,
}

impl Parse for NamespaceDeriveArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let inner;
        syn::parenthesized!(inner in input);

        let mut archive_tr: Option<(syn::Ident, ArchiveTrait)> = None;
        let mut serialize = None;
        let mut deserialize = None;
        let mut check_bytes = None;
        let mut singleton = None;

        while !inner.is_empty() {
            let ident: syn::Ident = inner.parse()?;
            let ident_name = ident.to_string();

            let duplicate = match ident_name.as_str() {
                stringify!(Archive) => archive_tr
                    .replace((ident.clone(), inner.parse()?))
                    .is_some(),
                stringify!(Serialize) => serialize.replace(Serialize(ident.clone())).is_some(),
                stringify!(Deserialize) => {
                    deserialize.replace(Deserialize(ident.clone())).is_some()
                }
                stringify!(CheckBytes) => check_bytes.replace(CheckBytes(ident.clone())).is_some(),
                stringify!(Singleton) => singleton.replace(Singleton(ident.clone())).is_some(),
                _ => {
                    return Err(syn::Error::new_spanned(
                        ident,
                        format!(
                            "{ident_name} is not a trait that can be automatically derived for this type. Please try adding {ident_name} as a supertrait requirement instead."
                        ),
                    ));
                }
            };

            if duplicate {
                return Err(syn::Error::new_spanned(
                    ident,
                    format!("Duplicate listing for {ident_name} found. Please remove."),
                ));
            }

            // Optional trailing comma.
            if inner.is_empty() {
                break;
            }

            let _ = inner.parse::<syn::Token![,]>();
        }

        Ok(Self {
            archive_tr,
            serialize,
            deserialize,
            check_bytes,
            singleton,
        })
    }
}

pub struct Args {
    pub namespace_ty: NamespaceTy,
    pub derive: Option<(syn::Ident, NamespaceDeriveArgs)>,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            namespace_ty: input.parse()?,
            derive: {
                if input.is_empty() {
                    None
                } else {
                    let _: syn::Token![,] = input.parse()?;
                    let derive_ident: syn::Ident = input.parse()?;

                    if derive_ident != "derive" {
                        return Err(syn::Error::new_spanned(
                            &derive_ident,
                            format!(
                                "{derive_ident} is not a recognized attribute for #[Namespace(..)]. Only `derive` is currently supported."
                            ),
                        ));
                    }

                    Some((derive_ident, input.parse()?))
                }
            },
        })
    }
}
