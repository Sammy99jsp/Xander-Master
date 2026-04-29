use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::punctuated::Punctuated;

use crate::namespace::parsing::NamespaceTy;

pub mod archive;
pub mod check_bytes;
pub mod derive_helper;
pub mod deserialize;
pub mod identity;
pub mod serialize;
pub mod singleton;

macro_rules! path_wrapper {
    ($(pub struct $id: ident;)*) => {
        $(
            #[repr(transparent)]
            pub struct $id(pub syn::Path);

            impl quote::ToTokens for $id {
                fn to_tokens(&self, tokens: &mut TokenStream) {
                    self.0.to_tokens(tokens);
                }
            }
        )*
    };
}

macro_rules! implementations {
    {
        $(#[$($a: tt)*])*
        pub struct $id: ident {
            $($p: ident),* $(,)?
        }
    } => {
        $(#[$($a)*])*
        pub struct $id {
            $(pub $p: Option<TokenStream>,)*
        }

        impl ToTokens for $id {
            fn to_tokens(&self, tokens: &mut TokenStream) {
                let Self { $($p,)* } = self;

                $(if let Some($p) = $p {
                    $p.to_tokens(tokens)
                })*
            }
        }
    };
}

implementations! {
    #[derive(Default)]
    pub struct Implementations {
        namespace,
        into_namespace,

        pointee_tr,
        pointee_archived,

        // Archive Trait itself.
        archive_tr,

        archive_unsized,
        archive_pointee,

        serialize_unsized,
        deserialize_unsized,

        check_bytes_archived,

        layout_raw_archived,
        layout_raw_tr,

        singleton_tr,

        derive_helper,
    }
}

impl Implementations {
    pub fn implement<I: Impl>(&mut self, impl_: I) {
        if let slot @ None = I::find(self) {
            slot.replace(impl_.generate());
        }
    }
}

pub trait Impl: Sized {
    fn find(impls: &mut Implementations) -> &mut Option<TokenStream>;
    fn generate(self) -> TokenStream;
}

pub fn supertrait<const N: usize>(path: syn::Path, lts: [syn::Lifetime; N]) -> syn::TypeParamBound {
    syn::TypeParamBound::Trait(syn::TraitBound {
        paren_token: Default::default(),
        modifier: syn::TraitBoundModifier::None,
        lifetimes: {
            if !lts.is_empty() {
                Some(syn::BoundLifetimes {
                    for_token: Default::default(),
                    lt_token: Default::default(),
                    gt_token: Default::default(),
                    lifetimes: Punctuated::from_iter(lts.map(|lifetime| {
                        syn::GenericParam::Lifetime(syn::LifetimeParam {
                            attrs: Default::default(),
                            colon_token: Default::default(),
                            bounds: Default::default(),
                            lifetime,
                        })
                    })),
                })
            } else {
                None
            }
        },
        path,
    })
}

path_wrapper! {
    pub struct NamespaceSt;
    pub struct NamespaceTr;
    pub struct ArchivedNamespaceTr;
}

pub mod prelude {
    pub use super::{
        ArchivedNamespaceTr, Impl, Implementations, NamespaceSt, NamespaceTr, supertrait,
    };
    pub(crate) use crate::utils::{self, PathExt as _, Respan as _, paths::*};
    pub use proc_macro2::{Span, TokenStream};
    pub use quote::quote;
}

pub struct PointeeImpl<'a, T> {
    pub target: &'a T,
    pub span: Span,
}

pub struct LayoutRawImpl<'a, T> {
    pub target: &'a T,
    pub span: Span,
}

impl<'a> From<&'a NamespaceTy> for NamespaceSt {
    fn from(value: &'a NamespaceTy) -> Self {
        match value {
            NamespaceTy::New { ident, .. } => Self(ident.clone().into()),
            NamespaceTy::Existing { path, .. } => Self(path.clone()),
        }
    }
}

impl<'a> From<&'a syn::ItemTrait> for NamespaceTr {
    fn from(value: &'a syn::ItemTrait) -> Self {
        Self(value.ident.clone().into())
    }
}
