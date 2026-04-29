pub mod impls;
pub mod parsing;

pub use parsing::Args;
use proc_macro2::{Span, TokenStream};
use quote::quote;

use crate::{
    namespace::{
        impls::{
            ArchivedNamespaceTr, Implementations, LayoutRawImpl, NamespaceSt, NamespaceTr,
            PointeeImpl,
            archive::{ArchivePointeeImpl, ArchiveUnsizedImpl, ArchivedTrait},
            check_bytes::CheckBytesArchivedImpl,
            derive_helper::DeriveHelper,
            deserialize::DeserializeUnsizedImpl,
            identity::{IntoNamespaceImpl, NamespaceImpl},
            serialize::SerializeUnsizedImpl,
            singleton::SingletonImpl,
            supertrait,
        },
        parsing::NamespaceDeriveArgs,
    },
    utils::{PathExt, paths::krate},
};

fn default_archive_trait_name(tr: &NamespaceTr) -> syn::Ident {
    syn::Ident::new(
        &format!("Archived{}", tr.0.get_ident().unwrap()),
        Span::call_site(),
    )
}

impl Args {
    pub fn process_for(self, mut tr: syn::ItemTrait) -> TokenStream {
        let Self {
            namespace_ty,
            derive,
        } = self;
        let mut impls = Implementations::default();

        impls.implement(NamespaceImpl::from(&namespace_ty));

        let ns_ident = NamespaceSt::from(&namespace_ty);
        let tr_ident = NamespaceTr::from(&tr);

        impls.implement(IntoNamespaceImpl::new(&tr_ident, &ns_ident));

        let ns_ty = ns_ident
            .0
            .with_last(|p| p.set_span(Span::call_site()))
            .to_type();

        // Add IdentityBase<NS> requirement.
        tr.supertraits
            .push(supertrait(krate::IdentityBase(ns_ty.clone()), []));

        let Some((
            derive_ident,
            NamespaceDeriveArgs {
                archive_tr: archive_tr_args,
                serialize,
                deserialize,
                check_bytes,
                singleton,
            },
        )) = derive
        else {
            return quote! {
                #tr
                #impls
            };
        };

        // Derive helper macro documentation:
        impls.implement(DeriveHelper {
            span: derive_ident.span(),
        });

        let mut archived_tr: Option<syn::ItemTrait> = None;

        let mut archived_tr_ident: Option<ArchivedNamespaceTr> = None;
        if let Some((archive_ident, archive_tr)) = archive_tr_args {
            let span = archive_tr
                .span()
                .and_then(|s| s.join(archive_ident.span()))
                .unwrap_or(archive_ident.span());

            let path = match archive_tr {
                parsing::ArchiveTrait::New(_, ident) => {
                    // Make a new archive trait.
                    let ident = ident.unwrap_or_else(|| default_archive_trait_name(&tr_ident));

                    archived_tr = Some(ArchivedTrait::create(span, &ident, &tr));

                    ident.into()
                }
                parsing::ArchiveTrait::Existing { path, .. } => path,
            };

            archived_tr_ident = Some(ArchivedNamespaceTr(path));

            let archive_tr = archived_tr_ident.as_ref().unwrap();
            // Add ArchiveUnsized implementation.
            impls.implement(ArchiveUnsizedImpl {
                span,
                tr: &tr_ident,
                archive_tr,
                archive_ident: &archive_ident,
            });

            // Add ArchiveUnsized's prerequisites

            // dyn ArchiveTr: ArchivePointee
            impls.implement(ArchivePointeeImpl {
                span,
                tr: &tr_ident,
                archive_tr,
            });

            // dyn ArchiveTr: Pointee
            impls.implement(PointeeImpl {
                target: archive_tr,
                span,
            });

            // dyn Tr: Pointee
            impls.implement(PointeeImpl {
                target: &tr_ident,
                span,
            });
        }

        if let Some(serialize) = serialize {
            // Add DynSerialize<'a> requirement.
            let lt = syn::Lifetime::new("'__a", Span::call_site());
            tr.supertraits
                .push(supertrait(krate::DynSerializeUnsized(lt.clone()), [lt]));

            // Add SerializesAs requirement.
            tr.supertraits
                .push(supertrait(krate::SerializesAs(ns_ty.clone()), []));

            // Add SerializeUnsized implementation
            impls.implement(SerializeUnsizedImpl {
                span: serialize.0.span(),
                tr: &tr_ident,
            });
        }

        if let Some(deserialize) = deserialize {
            tr.supertraits
                .push(supertrait(krate::DeserializesAs(ns_ty.clone()), []));

            // Add supertrait requirement to ArchivedTr, if we define it.
            if let Some(archived_tr) = archived_tr.as_mut() {
                archived_tr.supertraits.push(supertrait(
                    krate::Registered(krate::Deserializing.to_type()),
                    [],
                ))
            }

            impls.implement(DeserializeUnsizedImpl {
                span: deserialize.0.span(),
                tr: &tr_ident,
            });
        }

        if let Some(check_bytes) = check_bytes {
            if let Some(archived_tr) = &mut archived_tr {
                let lt = syn::Lifetime::new("'__a", Span::call_site());
                archived_tr
                    .supertraits
                    .push(supertrait(krate::DynCheckBytes(lt.clone()), [lt]));
            }

            impls.implement(CheckBytesArchivedImpl {
                tr: &tr_ident,
                span: check_bytes.0.span(),
            });

            if let Some(archived_ident) = &archived_tr_ident {
                impls.implement(LayoutRawImpl {
                    target: archived_ident,
                    span: check_bytes.0.span(),
                });
            }

            impls.implement(LayoutRawImpl {
                target: &tr_ident,
                span: check_bytes.0.span(),
            });
        }

        if let Some(singleton) = singleton {
            // Add supertrait requirement dyn Tr: Registered<StoredSingleton>
            tr.supertraits.push(supertrait(
                krate::Registered(krate::StoredSingleton.to_type()),
                [],
            ));

            impls.implement(SingletonImpl {
                tr: &tr_ident,
                span: singleton.0.span(),
            });

            impls.implement(PointeeImpl {
                target: &tr_ident,
                span: singleton.0.span(),
            });
        }

        quote! {
            #tr
            #archived_tr
            #impls
        }
    }
}
