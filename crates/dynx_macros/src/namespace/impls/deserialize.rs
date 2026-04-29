use super::prelude::*;

pub struct DeserializeUnsizedImpl<'a> {
    pub span: Span,
    pub tr: &'a NamespaceTr,
}

impl<'a> Impl for DeserializeUnsizedImpl<'a> {
    fn find(impls: &mut Implementations) -> &mut Option<TokenStream> {
        &mut impls.deserialize_unsized
    }

    fn generate(self) -> TokenStream {
        let Self { tr, span } = self;
        let d = utils::ty_param("D");

        let deserialize_unsized = rkyv::DeserializeUnsized.with_last(|p| p.set_span(span));
        let archive_unsized = &rkyv::ArchiveUnsized;
        let fallible = &rkyv::Fallible;
        let pointee = &rkyv::Pointee;

        let utils_deserialize_unsized = &krate::utils_deserialize_unsized;
        let utils_deserialize_metadata = &krate::utils_deserialize_metadata;

        quote! {
            #[automatically_derived]
            impl<#d> #deserialize_unsized<dyn #tr, #d>
                for <dyn #tr as #archive_unsized>::Archived
            where
                #d: #fallible + ?Sized,
                #d::Error: 'static,
            {
                unsafe fn deserialize_unsized(
                    &self,
                    deserializer: &mut #d,
                    out: *mut dyn #tr,
                ) -> Result<(), <#d as #fallible>::Error> {
                    unsafe { #utils_deserialize_unsized(self, deserializer, out) }
                }
                fn deserialize_metadata(&self) -> <dyn #tr as #pointee>::Metadata {
                    #utils_deserialize_metadata::<dyn #tr>(self)
                }
            }
        }
        .respan(span)
    }
}
