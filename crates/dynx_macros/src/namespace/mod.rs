use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::{parse::Parse, punctuated::Punctuated};

use crate::paths::{self, ty_for};

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
    New(Option<syn::Ident>),
    Existing {
        _at: syn::Token![@],
        path: syn::Path,
    },
}

impl Parse for ArchiveTrait {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if !input.peek(syn::token::Paren) {
            return Ok(Self::New(None));
        }

        let inner;
        syn::parenthesized!(inner in input);

        match inner.peek(syn::Token![@]) {
            true => Ok(Self::Existing {
                _at: inner.parse()?,
                path: inner.parse()?,
            }),
            false => Ok(Self::New(if inner.is_empty() {
                None
            } else {
                inner.parse()?
            })),
        }
    }
}

fn name_for_archive<T: std::fmt::Display>(name: &T) -> String {
    format!("Archived{name}")
}

#[derive(Debug)]
pub struct Serialize(syn::Ident);

#[derive(Debug)]
pub struct Deserialize(syn::Ident);

#[derive(Debug)]
pub struct CheckBytes(syn::Ident);

#[derive(Debug)]
pub struct Singleton(syn::Ident);

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
                                "{derive_ident} is not a reconised attribute for #[Namespace(..)]. Only `derive` is currently supported."
                            ),
                        ));
                    }

                    Some((derive_ident, input.parse()?))
                }
            },
        })
    }
}

impl Args {
    pub fn process_for(self, mut tr: syn::ItemTrait) -> TokenStream {
        // Generate types, impls.
        let mut namespace_ty = TokenStream::new();

        let tr_ident = {
            let mut tr = tr.ident.clone();
            tr.set_span(Span::call_site());
            tr
        };

        if let NamespaceTy::New { id, ident, .. } = &self.namespace_ty {
            let namespace_tr_path = paths::krate::Namespace();
            let mut ident_copy = ident.clone();
            ident_copy.set_span(Span::call_site());
            let id_ident = syn::Ident::new("ID", id.span());

            namespace_ty = quote! {
                pub struct #ident;
                #[automatically_derived]
                impl #namespace_tr_path for #ident_copy {
                    const #id_ident: &'static str = #id;
                }
            }
        }

        let namespace = match self.namespace_ty {
            NamespaceTy::New { mut ident, .. } => {
                ident.set_span(Span::call_site());
                ident
            }
            .into(),
            NamespaceTy::Existing { path, .. } => path,
        };

        let tr_ns_type = syn::Type::Path(syn::TypePath {
            qself: None,
            path: namespace.clone(),
        });

        // Add IdentityBase<NS> supertrait requirement for $tr.
        {
            let supertraits = &mut tr.supertraits;

            supertraits.push(syn::TypeParamBound::Trait(syn::TraitBound {
                paren_token: None,
                modifier: syn::TraitBoundModifier::None,
                lifetimes: None,
                path: {
                    let mut path = paths::krate::IdentityBase();
                    path.segments.last_mut().unwrap().arguments =
                        syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                            colon2_token: None,
                            lt_token: <syn::Token![<]>::default(),
                            args: Punctuated::from_iter([syn::GenericArgument::Type(
                                syn::Type::Path(syn::TypePath {
                                    qself: None,
                                    path: namespace.clone(),
                                }),
                            )]),
                            gt_token: <syn::Token![>]>::default(),
                        });
                    path
                },
            }));
        }

        let into_namespace_impl = {
            let into_namespace = paths::krate::IntoNamespace();

            quote! {
                #[automatically_derived]
                impl #into_namespace for dyn #tr_ident {
                    type Namespace = #namespace;
                }
            }
        };

        // Process derive requests, otherwise end early.
        let Some((derive_ident, derive)) = self.derive else {
            return quote! {
                #tr
                #namespace_ty
                #into_namespace_impl
            };
        };

        let mut archive_unsized_impl = TokenStream::new();
        let mut archive_trait_and_impls = TokenStream::new();
        if let Some((archive_ident, archive)) = derive.archive_tr {
            let (archived_trait, archived_path) = match archive {
                ArchiveTrait::Existing { path, .. } => (TokenStream::new(), path),
                ArchiveTrait::New(ident) => {
                    let ident = ident.unwrap_or_else(|| {
                        syn::Ident::new(&name_for_archive(&tr.ident), Span::call_site())
                    });

                    let mut archived_trait = syn::ItemTrait {
                        attrs: Vec::new(),
                        vis: tr.vis.clone(),
                        unsafety: tr.unsafety,
                        auto_token: None,
                        restriction: None,
                        trait_token: <syn::Token![trait]>::default(),
                        ident: ident.clone(),
                        generics: syn::Generics::default(),
                        colon_token: Some(<syn::Token![:]>::default()),
                        supertraits: Punctuated::new(),
                        brace_token: syn::token::Brace::default(),
                        items: Vec::new(),
                    };

                    archived_trait
                        .supertraits
                        .push(syn::TypeParamBound::Trait(syn::TraitBound {
                            paren_token: None,
                            modifier: syn::TraitBoundModifier::None,
                            lifetimes: None,
                            path: paths::rkyv::Portable(),
                        }));

                    archived_trait
                        .supertraits
                        .push(syn::TypeParamBound::Trait(syn::TraitBound {
                            paren_token: None,
                            modifier: syn::TraitBoundModifier::None,
                            lifetimes: None,
                            path: paths::krate::Registered(ty_for(paths::krate::Archiving())),
                        }));

                    if derive.deserialize.is_some() {
                        archived_trait.supertraits.push(syn::TypeParamBound::Trait(
                            syn::TraitBound {
                                paren_token: None,
                                modifier: syn::TraitBoundModifier::None,
                                lifetimes: None,
                                path: paths::krate::Registered(ty_for(
                                    paths::krate::Deserializing(),
                                )),
                            },
                        ));
                    }

                    if derive.check_bytes.is_some() {
                        let lt = syn::Lifetime::new("'__a", Span::call_site());
                        archived_trait.supertraits.push(syn::TypeParamBound::Trait(
                            syn::TraitBound {
                                paren_token: None,
                                modifier: syn::TraitBoundModifier::None,
                                lifetimes: Some(syn::BoundLifetimes {
                                    for_token: <syn::Token![for]>::default(),
                                    lt_token: <syn::Token![<]>::default(),
                                    gt_token: <syn::Token![>]>::default(),
                                    lifetimes: Punctuated::from_iter([
                                        syn::GenericParam::Lifetime(syn::LifetimeParam {
                                            attrs: Vec::new(),
                                            lifetime: lt.clone(),
                                            colon_token: None,
                                            bounds: Punctuated::new(),
                                        }),
                                    ]),
                                }),
                                path: {
                                    let mut p = paths::krate::DynCheckBytes();
                                    p.segments.last_mut().unwrap().arguments =
                                        syn::PathArguments::AngleBracketed(
                                            syn::AngleBracketedGenericArguments {
                                                colon2_token: None,
                                                lt_token: <syn::Token![<]>::default(),
                                                gt_token: <syn::Token![>]>::default(),
                                                args: Punctuated::from_iter([
                                                    syn::GenericArgument::Lifetime(lt),
                                                ]),
                                            },
                                        );
                                    p
                                },
                            },
                        ));
                    }

                    (archived_trait.to_token_stream(), ident.into())
                }
            };

            let mut deserialize_impl = TokenStream::new();
            if let Some(Deserialize(deserialize_ident)) = derive.deserialize {
                tr.supertraits
                    .push(syn::TypeParamBound::Trait(syn::TraitBound {
                        paren_token: None,
                        modifier: syn::TraitBoundModifier::None,
                        lifetimes: None,
                        path: paths::krate::DeserializesAs(tr_ns_type.clone()),
                    }));

                let archive_unsized = paths::rkyv::ArchiveUnsized();

                let deserialize_unsized = paths::rkyv::DeserializeUnsized();
                let dyn_deserializer = paths::krate::DynDeserializer();

                let fallible = paths::rkyv::Fallible();
                let registry = paths::krate::REGISTRY();
                let ptr_meta = paths::rkyv::ptr_meta();

                let layout_raw = paths::rkyv::LayoutRaw();
                let pointee = paths::rkyv::Pointee();
                let layout = paths::std::Layout();
                let layout_error = paths::std::LayoutError();

                let span = deserialize_ident.span();

                let deserialize_unsized_impl_template =
                    |g: TokenStream, ty: TokenStream, mapper: TokenStream| {
                        quote_spanned! { span =>
                            #[automatically_derived]
                            impl #g #deserialize_unsized<dyn #tr_ident, #ty> for <dyn #tr_ident as #archive_unsized>::Archived
                            where
                                #ty: #fallible + 'static
                            {
                                unsafe fn deserialize_unsized(
                                    &self,
                                    deserializer: &mut #ty,
                                    out: *mut dyn #tr_ident,
                                ) -> Result<(), <#ty as #fallible>::Error> {
                                    let (this, archive_meta) = #ptr_meta::to_raw_parts(self);
                                    let record = #registry.lookup_by_archive::<dyn #tr_ident>(archive_meta).expect("To be registered!");

                                    let (out, _) = #ptr_meta::to_raw_parts_mut(out);
                                    unsafe { (record.deserializing.unwrap().erased_deserialize_fn)(this, deserializer, out) #mapper }
                                }

                                fn deserialize_metadata(&self) -> <dyn #tr_ident as #ptr_meta::Pointee>::Metadata {
                                    let (_, archive_meta) = #ptr_meta::to_raw_parts(self);
                                    let record = #registry.lookup_by_archive::<dyn #tr_ident>(archive_meta).expect("To be registered!");
                                    unsafe { record.archiving.unwrap().cast::<dyn #tr_ident>().meta }
                                }
                            }
                        }
                    };

                let layout_raw = quote_spanned! { span =>
                    #[automatically_derived]
                    impl #layout_raw for dyn #tr_ident {
                        fn layout_raw(
                            metadata: <Self as #pointee>::Metadata,
                        ) -> Result<#layout, #layout_error> {
                            Ok(metadata.layout())
                        }
                    }
                };

                let deserialize_unsized_impl_blanket = deserialize_unsized_impl_template(
                    quote! {<D>},
                    quote! {D},
                    quote! {.map_err(|err| err.downcast::<D::Error>().unwrap())},
                );
                let deserialize_unsized_impl_dyn = deserialize_unsized_impl_template(
                    quote! {},
                    quote! {(dyn #dyn_deserializer + 'static)},
                    quote! {},
                );

                deserialize_impl = quote! {
                    #deserialize_unsized_impl_blanket
                    #deserialize_unsized_impl_dyn
                    #layout_raw
                }
            }

            let mut check_bytes_impl = TokenStream::new();
            if let Some(CheckBytes(check_bytes_ident)) = derive.check_bytes {
                let check_bytes = paths::rkyv::CheckBytes();

                let archive_unsized = paths::rkyv::ArchiveUnsized();
                let fallible = paths::rkyv::Fallible();
                let dyn_byte_checker = paths::krate::DynByteChecker();
                let dyn_check_bytes = paths::krate::DynCheckBytes();
                let dyn_error = paths::krate::DynError();

                let layout_raw = paths::rkyv::LayoutRaw();
                let pointee = paths::rkyv::Pointee();
                let layout = paths::std::Layout();
                let layout_error = paths::std::LayoutError();

                let span = check_bytes_ident.span();
                let mut check_bytes_i1: syn::ItemImpl = syn::parse2(quote_spanned! { span =>
                    #[automatically_derived]
                    unsafe impl<C> #check_bytes<C> for <dyn #tr_ident as #archive_unsized>::Archived
                    where
                    C: #fallible + #dyn_byte_checker,
                    C::Error: 'static,
                    {
                        unsafe fn check_bytes(
                            value: *const Self,
                            context: &mut C,
                        ) -> Result<(), C::Error>
                        {
                            unsafe {
                                #dyn_check_bytes::check_bytes(value.as_ref().unwrap(), context)
                                .map_err(|err| err.downcast::<C::Error>().unwrap())
                            }
                        }
                    }
                })
                .unwrap();

                check_bytes_i1
                    .trait_
                    .as_mut()
                    .unwrap()
                    .1
                    .segments
                    .last_mut()
                    .unwrap()
                    .ident
                    .set_span(check_bytes_ident.span());

                let layout_raw = quote_spanned! { span =>
                    #[automatically_derived]
                    impl #layout_raw for <dyn #tr_ident as #archive_unsized>::Archived {
                        fn layout_raw(
                            metadata: <Self as #pointee>::Metadata,
                        ) -> Result<#layout, #layout_error> {
                            Ok(metadata.layout())
                        }
                    }
                };

                check_bytes_impl = quote_spanned! { span =>
                    #check_bytes_i1

                    #[automatically_derived]
                    unsafe impl<'a> #check_bytes<dyn #dyn_byte_checker + 'a> for <dyn #tr_ident as #archive_unsized>::Archived
                    {
                        unsafe fn check_bytes(
                            value: *const Self,
                            context: &mut (dyn #dyn_byte_checker + 'a),
                        ) -> Result<(), #dyn_error>
                        {
                            unsafe {
                                #dyn_check_bytes::check_bytes(value.as_ref().unwrap(), context)
                            }
                        }
                    }

                    #layout_raw
                };
            }

            let pointee = paths::rkyv::Pointee();
            let dyn_metadata = paths::rkyv::DynMetadata(paths::self_ty());
            let archive_pointee = paths::rkyv::ArchivePointee();
            let archived_local_id = paths::krate::ArchivedLocalId();
            let registry = paths::krate::REGISTRY();

            // De-span the ident to prevent multiple locations on pop-up.
            let archived_path = {
                let mut path = archived_path.clone();
                path.segments
                    .last_mut()
                    .unwrap()
                    .ident
                    .set_span(Span::call_site());
                path
            };

            archive_trait_and_impls = quote! {
                #archived_trait

                #[automatically_derived]
                unsafe impl #pointee for dyn #archived_path {
                    type Metadata = #dyn_metadata;
                }

                #[automatically_derived]
                impl #archive_pointee for dyn #archived_path {
                    type ArchivedMetadata = #archived_local_id;

                    fn pointer_metadata(
                        archived: &Self::ArchivedMetadata,
                    ) -> <Self as #pointee>::Metadata {
                        let record = #registry
                            .lookup::<dyn #tr_ident>(archived.as_str())
                            .expect("Should be registered");

                        unsafe { record.archiving.unwrap().cast::<dyn #tr_ident>().archived }
                    }
                }

                #deserialize_impl

                #check_bytes_impl
            };

            archive_unsized_impl = {
                let pointee = paths::rkyv::Pointee();
                let dyn_metadata = paths::rkyv::DynMetadata(paths::self_ty());

                let mut archive_unsized = paths::rkyv::ArchiveUnsized();
                archive_unsized
                    .segments
                    .last_mut()
                    .unwrap()
                    .ident
                    .set_span(archive_ident.span());

                let archived_meta = paths::rkyv::ArchivedMetadata(paths::self_ty());
                let local_id = paths::krate::ArchivedLocalId();
                let identity_base = paths::krate::IdentityBase();
                quote! {
                    #[automatically_derived]
                    unsafe impl<'a> #pointee for dyn #tr_ident + 'a {
                        type Metadata = #dyn_metadata;
                    }

                    #[automatically_derived]
                    impl<'a> #archive_unsized for dyn #tr_ident + 'a {
                        type Archived = dyn #archived_path;

                        fn archived_metadata(&self) -> #archived_meta {
                            #local_id::new(#identity_base::local_id(self))
                        }
                    }
                }
            };
        }

        let mut archive_serialize_impl = TokenStream::new();
        if let Some(Serialize(_serialize_ident)) = derive.serialize {
            let lt_a = syn::Lifetime::new("'__a", Span::call_site());
            tr.supertraits
                .push(syn::TypeParamBound::Trait(syn::TraitBound {
                    paren_token: None,
                    modifier: syn::TraitBoundModifier::None,
                    lifetimes: Some(syn::BoundLifetimes {
                        for_token: <syn::Token![for]>::default(),
                        lt_token: <syn::Token![<]>::default(),
                        gt_token: <syn::Token![>]>::default(),
                        lifetimes: Punctuated::from_iter([syn::GenericParam::Lifetime(
                            syn::LifetimeParam {
                                attrs: Vec::new(),
                                lifetime: lt_a.clone(),
                                colon_token: None,
                                bounds: Punctuated::new(),
                            },
                        )]),
                    }),
                    path: {
                        let mut p = paths::krate::DynSerializeUnsized();
                        p.segments.last_mut().unwrap().arguments =
                            syn::PathArguments::AngleBracketed(
                                syn::AngleBracketedGenericArguments {
                                    colon2_token: None,
                                    lt_token: <syn::Token![<]>::default(),
                                    args: Punctuated::from_iter([syn::GenericArgument::Lifetime(
                                        lt_a,
                                    )]),
                                    gt_token: <syn::Token![>]>::default(),
                                },
                            );
                        p
                    },
                }));

            tr.supertraits
                .push(syn::TypeParamBound::Trait(syn::TraitBound {
                    paren_token: None,
                    modifier: syn::TraitBoundModifier::None,
                    lifetimes: None,
                    path: paths::krate::SerializesAs(tr_ns_type.clone()),
                }));

            let s = syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::Ident::new("S", Span::call_site()).into(),
            });

            let serialize = paths::rkyv::SerializeUnsized();

            let fallible = paths::rkyv::Fallible();
            let writer = paths::rkyv::Writer();
            let sharing = paths::rkyv::Sharing();
            let allocator = paths::rkyv::Allocator();
            let error = paths::core::Error();
            let dyn_serialize = paths::krate::DynSerializeUnsized();
            let dyn_serializer = paths::krate::DynSerializer();

            archive_serialize_impl = quote! {
                #[automatically_derived]
                impl<'a, #s> #serialize<#s> for dyn #tr_ident + 'a
                where
                    #s: #fallible + #writer + #sharing + #allocator,
                    #s::Error: #error + Send + Sync + 'static,
                {
                    fn serialize_unsized(&self, serializer: &mut #s) -> Result<usize, <#s as #fallible>::Error> {
                        #dyn_serialize::serialize_unsized(self, serializer).map_err(|err| err.downcast().unwrap())
                    }
                }

                #[automatically_derived]
                impl<'a, 'b> #serialize<dyn #dyn_serializer + 'b> for dyn #tr_ident + 'a {
                    fn serialize_unsized(
                        &self,
                        serializer: &mut (dyn #dyn_serializer + 'b),
                    ) -> Result<usize, <dyn #dyn_serializer + 'b  as #fallible>::Error>
                    {
                        #dyn_serialize::serialize_unsized(self, serializer)
                    }
                }
            }
        }

        let mut singleton_impl = TokenStream::new();
        if let Some(Singleton(singleton_ident)) = derive.singleton {
            tr.supertraits
                .push(syn::TypeParamBound::Trait(syn::TraitBound {
                    paren_token: Default::default(),
                    modifier: syn::TraitBoundModifier::None,
                    lifetimes: None,
                    path: paths::krate::Registered(ty_for({
                        let mut p = paths::krate::StoredSingleton();
                        p.segments
                            .last_mut()
                            .unwrap()
                            .ident
                            .set_span(singleton_ident.span());
                        p
                    })),
                }));

            let pointee = paths::rkyv::Pointee();
            let dyn_metadata = paths::rkyv::DynMetadata(paths::self_ty());
            let singleton = paths::krate::Singleton();

            singleton_impl = quote! {
                unsafe impl #pointee for dyn #tr_ident {
                    type Metadata = #dyn_metadata;
                }

                impl #singleton for dyn #tr_ident {}
            }
        }

        let derive_span = {
            let mut derive_macro = paths::krate::derive();
            // derive_ident.set_span(derive_macro.segments.last().unwrap().span());
            derive_macro
                .segments
                .last_mut()
                .unwrap()
                .ident
                .set_span(derive_ident.span());
            derive_macro.leading_colon = None;

            quote! {
                #[allow(unused_imports)]
                const _: () = {
                    use #derive_macro;
                };
            }
        };

        quote! {
            #tr
            #archive_serialize_impl
            #singleton_impl
            #namespace_ty
            #into_namespace_impl
            #archive_unsized_impl
            #archive_trait_and_impls
            #derive_span
        }
    }
}

// ArchiveUnsized: ptr_meta::Pointee + <Self::Archived>: ArchivePointee + Portable
// SerializeUnsized: ArchiveUnsized
// DeserializeUnsized: LayoutRaw

//
