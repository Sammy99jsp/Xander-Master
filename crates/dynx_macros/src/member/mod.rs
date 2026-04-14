use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{parse::Parse, punctuated::Punctuated, spanned::Spanned};

use crate::paths::{self, ty_for};

pub struct Args {
    local_id: syn::Expr,
    register: Option<Register>,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let local_id = input.parse()?;

        if !input.is_empty() {
            let _: syn::Token![,] = input.parse()?;
        }

        let register = if input.is_empty() {
            None
        } else {
            Some(input.parse()?)
        };

        if !input.is_empty() {
            let _: syn::Token![,] = input.parse()?;
        }

        Ok(Self { local_id, register })
    }
}

impl Args {
    pub fn process_for(&self, impl_: syn::ItemImpl) -> TokenStream {
        let Some((None, tr, ..)) = &impl_.trait_ else {
            return syn::Error::new_spanned(impl_, "Expected a trait implementation here.")
                .into_compile_error();
        };

        let self_ty = {
            let mut self_ty = impl_.self_ty.as_ref().clone();
            if let syn::Type::Path(syn::TypePath { path, .. }) = &mut self_ty {
                path.segments
                    .iter_mut()
                    .for_each(|seg| seg.ident.set_span(Span::call_site()));
            }
            self_ty
        };

        let tr = {
            let mut tr = tr.clone();
            tr.segments
                .iter_mut()
                .for_each(|seg| seg.ident.set_span(Span::call_site()));
            tr
        };

        let dyn_tr = syn::Type::TraitObject(syn::TypeTraitObject {
            dyn_token: Some(Default::default()),
            bounds: Punctuated::from_iter([syn::TypeParamBound::Trait(syn::TraitBound {
                paren_token: None,
                modifier: syn::TraitBoundModifier::None,
                lifetimes: None,
                path: tr.clone(),
            })]),
        });

        // Implement Identity
        let identity_impl = {
            let identity = paths::krate::Identity();
            let local_id = &self.local_id;
            let local_id_ident = syn::Ident::new("LOCAL_ID", local_id.span());

            quote! {
                impl #identity for #self_ty {
                    type Parent = #dyn_tr;
                    const #local_id_ident: &'static str = #local_id;
                }
            }
        };

        let mut register_helper = TokenStream::new();
        let mut registered_archive = TokenStream::new();
        let mut registered_deserialize = TokenStream::new();
        let mut registered_singleton = TokenStream::new();

        if let Some(Register {
            register_kw,
            deserialize,
            archive,
            singleton,
            ..
        }) = &self.register
        {
            // Documentation for the `register` helper for the macro.
            let mut register = paths::krate::register();
            register
                .segments
                .last_mut()
                .unwrap()
                .ident
                .set_span(register_kw.span());

            register_helper = quote! {
                #[allow(unused_imports)]
                const _: () = {
                    use #register;
                };
            };

            // Implement Registered<Archiving>, if asked to.
            if let Some(Archive(archive_kw)) = archive {
                registered_archive = {
                    let registered_archive = paths::krate::Registered(ty_for({
                        let mut p = paths::krate::Archiving();
                        p.segments
                            .last_mut()
                            .unwrap()
                            .ident
                            .set_span(archive_kw.span());
                        p
                    }));

                    let archived = paths::rkyv::Archived(self_ty.clone());
                    let archiving = paths::krate::Archiving();
                    let record = paths::krate::Record();
                    let submit = paths::inventory::submit();

                    let span = archive_kw.span();
                    quote_spanned! { span =>
                        unsafe impl #registered_archive for #archived {}

                        #submit! {
                            #record::new::<#self_ty, #dyn_tr>(#archiving::new::<#self_ty, #dyn_tr>())
                        }
                    }
                };
            }

            // Implement Registered<Deserializing>, if asked to.
            if let Some(Deserialize(deserialize_kw)) = deserialize {
                registered_deserialize = {
                    let registered_deserialize = paths::krate::Registered(ty_for({
                        let mut p = paths::krate::Deserializing();
                        p.segments
                            .last_mut()
                            .unwrap()
                            .ident
                            .set_span(deserialize_kw.span());
                        p
                    }));

                    let archived = paths::rkyv::Archived(self_ty.clone());
                    let deserializing = paths::krate::Deserializing();
                    let record = paths::krate::Record();
                    let submit = paths::inventory::submit();

                    let span = deserialize_kw.span();
                    quote_spanned! { span =>
                        unsafe impl #registered_deserialize for #archived {}

                        #submit! {
                            #record::new::<#self_ty, #dyn_tr>(#deserializing::new::<#self_ty, #dyn_tr>())
                        }
                    }
                };
            }

            // Implement Registered<StoredSingleton>, if asked to.
            if let Some(Singleton(singleton_kw)) = singleton {
                let submit = paths::inventory::submit();

                let registered = paths::krate::Registered(ty_for(paths::krate::StoredSingleton()));
                let record = paths::krate::Record();
                let mut stored_singleton = paths::krate::StoredSingleton();

                stored_singleton
                    .segments
                    .last_mut()
                    .unwrap()
                    .ident
                    .set_span(singleton_kw.span());

                registered_singleton = quote! {
                    unsafe impl #registered for #self_ty {}

                    #submit! {
                        #record::new::<#self_ty, #dyn_tr>(#stored_singleton::new(&#self_ty as &#dyn_tr))
                    }
                }
            }
        }

        quote! {
            #impl_
            #identity_impl
            #register_helper
            #registered_archive
            #registered_deserialize
            #registered_singleton
        }
    }
}

pub struct Register {
    register_kw: syn::Ident,
    _paren: syn::token::Paren,
    deserialize: Option<Deserialize>,
    archive: Option<Archive>,
    singleton: Option<Singleton>,
}

fn parse_keyword<T: std::fmt::Display>(
    input: syn::parse::ParseStream,
    is_keyword: fn(&syn::Ident) -> bool,
    msg: T,
) -> syn::Result<syn::Ident> {
    input
        .parse()
        .map_err(|err| err.span())
        .and_then(|ident: syn::Ident| {
            if is_keyword(&ident) {
                Ok(ident)
            } else {
                Err(ident.span())
            }
        })
        .map_err(|span| syn::Error::new(span, msg))
}

impl Parse for Register {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let register_kw = parse_keyword(
            input,
            |ident| ident == "register",
            "Expected `register()` helper macro here.",
        )?;

        let inner;
        let _paren = syn::parenthesized!(inner in input);

        let input = &inner;

        let mut deserialize = None;
        let mut archive = None;
        let mut singleton = None;

        while !inner.is_empty() {
            let ident = parse_keyword(
                input,
                |_| true,
                "Expected `Deserialize` or `Archive` trait here.",
            )?;

            let replaced = match ident.to_string().as_str() {
                stringify!(Deserialize) => deserialize
                    .replace(Deserialize(ident.clone()))
                    .map(|_| ident.span()),
                stringify!(Archive) => archive
                    .replace(Archive(ident.clone()))
                    .map(|_| ident.span()),
                stringify!(Singleton) => singleton
                    .replace(Singleton(ident.clone()))
                    .map(|_| ident.span()),
                _ => None,
            };

            if let Some(replaced) = replaced {
                return Err(syn::Error::new(
                    replaced,
                    "You cannot specify traits multiple times.",
                ));
            }

            if !inner.is_empty() {
                let _: syn::Token![,] = inner.parse()?;
            }
        }

        Ok(Self {
            register_kw,
            _paren,
            deserialize,
            archive,
            singleton,
        })
    }
}

pub struct Deserialize(syn::Ident);

pub struct Archive(syn::Ident);

pub struct Singleton(syn::Ident);
