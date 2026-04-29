use crate::namespace::impls::LayoutRawImpl;

use super::prelude::*;

pub struct CheckBytesArchivedImpl<'a> {
    pub tr: &'a NamespaceTr,
    pub span: Span,
}

impl<'a> Impl for CheckBytesArchivedImpl<'a> {
    fn find(impls: &mut Implementations) -> &mut Option<TokenStream> {
        &mut impls.check_bytes_archived
    }

    fn generate(self) -> TokenStream {
        let Self { tr, span } = self;

        let c = utils::ty_param("C");

        let archive_unsized = &rkyv::ArchiveUnsized;
        let fallible = &rkyv::Fallible;
        let check_bytes = rkyv::CheckBytes(c.clone()).with_last(|p| p.set_span(span));

        let utils_check_bytes = &krate::utils_check_bytes;

        quote! {
            #[automatically_derived]
            unsafe impl<'a, #c> #check_bytes for <dyn #tr + 'a as #archive_unsized>::Archived
            where
                #c: #fallible + ?Sized,
                #c::Error: core::error::Error + Send + Sync + 'static,
            {
                unsafe fn check_bytes(
                    value: *const Self,
                    context: &mut #c,
                ) -> Result<(), <#c as #fallible>::Error> {
                    #utils_check_bytes(value, context)
                }
            }
        }
        .respan(span)
    }
}

impl<'a> Impl for LayoutRawImpl<'a, ArchivedNamespaceTr> {
    fn find(impls: &mut Implementations) -> &mut Option<TokenStream> {
        &mut impls.layout_raw_archived
    }

    fn generate(self) -> TokenStream {
        let Self {
            target: archived_tr,
            span,
        } = self;

        let layout_raw = &rkyv::LayoutRaw;
        let pointee = &rkyv::Pointee;

        quote! {
            impl #layout_raw for dyn #archived_tr + '_ {
                fn layout_raw(
                    metadata: <Self as #pointee>::Metadata,
                ) -> Result<std::alloc::Layout, std::alloc::LayoutError> {
                    Ok(metadata.layout())
                }
            }
        }
        .respan(span)
    }
}

impl<'a> Impl for LayoutRawImpl<'a, NamespaceTr> {
    fn find(impls: &mut Implementations) -> &mut Option<TokenStream> {
        &mut impls.layout_raw_tr
    }

    fn generate(self) -> TokenStream {
        let Self { target: tr, span } = self;

        let layout_raw = &rkyv::LayoutRaw;
        let pointee = &rkyv::Pointee;

        quote! {
            impl #layout_raw for dyn #tr + '_ {
                fn layout_raw(
                    metadata: <Self as #pointee>::Metadata,
                ) -> Result<std::alloc::Layout, std::alloc::LayoutError> {
                    Ok(metadata.layout())
                }
            }
        }
        .respan(span)
    }
}
