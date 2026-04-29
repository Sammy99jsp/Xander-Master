//! Traits relating to ArchiveUnsized and friends.

use syn::punctuated::Punctuated;

use super::prelude::*;

pub struct ArchivedTrait;

impl ArchivedTrait {
    pub fn create(_: Span, ident: &syn::Ident, tr: &syn::ItemTrait) -> syn::ItemTrait {
        syn::ItemTrait {
            attrs: Default::default(),
            vis: tr.vis.clone(),
            unsafety: tr.unsafety,
            auto_token: tr.auto_token,
            restriction: tr.restriction.clone(),
            trait_token: Default::default(),
            ident: ident.clone(),
            generics: tr.generics.clone(),
            colon_token: Some(Default::default()),
            supertraits: Punctuated::from_iter([
                supertrait(rkyv::Portable(), []),
                supertrait(krate::Registered(krate::Archiving.to_type()), []),
            ]),
            brace_token: Default::default(),
            items: Vec::new(),
        }
    }
}

pub struct ArchiveUnsizedImpl<'a> {
    pub span: Span,
    pub archive_ident: &'a syn::Ident,
    pub tr: &'a NamespaceTr,
    pub archive_tr: &'a ArchivedNamespaceTr,
}

impl<'a> Impl for ArchiveUnsizedImpl<'a> {
    fn find(impls: &mut Implementations) -> &mut Option<TokenStream> {
        &mut impls.archive_unsized
    }

    fn generate(self) -> TokenStream {
        let Self {
            tr,
            archive_tr,
            span,
            archive_ident,
        } = self;

        let archive_unsized = &rkyv::ArchiveUnsized.with_last(|s| s.set_span(archive_ident.span()));
        let archived_metadata = rkyv::ArchivedMetadata(utils::self_ty());

        let archived_local_id = &krate::ArchivedLocalId;
        let identity_base = &krate::IdentityBase;
        let archive_tr = archive_tr.erase_span();
        quote! {
            #[automatically_derived]
            impl<'a> #archive_unsized for dyn #tr + 'a {
                type Archived = dyn #archive_tr + 'a;
                fn archived_metadata(&self) -> #archived_metadata {
                    #archived_local_id::new(#identity_base::local_id(self))
                }
            }
        }
        .respan(span)
    }
}

impl<'a> Impl for super::PointeeImpl<'a, ArchivedNamespaceTr> {
    fn find(impls: &mut Implementations) -> &mut Option<TokenStream> {
        &mut impls.pointee_archived
    }

    fn generate(self) -> TokenStream {
        let Self {
            target: archive_tr,
            span,
        } = self;
        let pointee = &rkyv::Pointee;
        let dyn_metadata = &rkyv::DynMetadata(utils::self_ty());

        let archive_tr = archive_tr.erase_span();
        quote! {
            #[automatically_derived]
            unsafe impl #pointee for dyn #archive_tr + '_ {
                type Metadata = #dyn_metadata;
            }
        }
        .respan(span)
    }
}

impl<'a> Impl for super::PointeeImpl<'a, NamespaceTr> {
    fn find(impls: &mut Implementations) -> &mut Option<TokenStream> {
        &mut impls.pointee_tr
    }

    fn generate(self) -> TokenStream {
        let Self {
            target: archive_tr,
            span,
        } = self;
        let pointee = &rkyv::Pointee;
        let dyn_metadata = &rkyv::DynMetadata(utils::self_ty());

        let archive_tr = archive_tr.erase_span();

        quote! {
            #[automatically_derived]
            unsafe impl #pointee for dyn #archive_tr + '_ {
                type Metadata = #dyn_metadata;
            }
        }
        .respan(span)
    }
}

pub struct ArchivePointeeImpl<'a> {
    pub span: Span,
    pub tr: &'a NamespaceTr,
    pub archive_tr: &'a ArchivedNamespaceTr,
}

impl<'a> Impl for ArchivePointeeImpl<'a> {
    fn find(impls: &mut Implementations) -> &mut Option<TokenStream> {
        &mut impls.archive_pointee
    }

    fn generate(self) -> TokenStream {
        let Self {
            tr,
            archive_tr,
            span,
        } = self;

        let archive_pointee = &rkyv::ArchivePointee;
        let archived_local_id = &krate::ArchivedLocalId;
        let pointee = &rkyv::Pointee;

        let registry = &krate::REGISTRY;
        let archive_tr = archive_tr.erase_span();

        quote! {
            #[automatically_derived]
            impl #archive_pointee for dyn #archive_tr + '_ {
                type ArchivedMetadata = #archived_local_id;
                fn pointer_metadata(
                    archived: &Self::ArchivedMetadata,
                ) -> <Self as #pointee>::Metadata {
                    let record = #registry
                        .lookup_by_local::<dyn #tr>(*archived)
                        .expect("Should be registered");
                    unsafe { record.archiving.unwrap().cast::<dyn #tr>().archived }
                }
            }
        }
        .respan(span)
    }
}
