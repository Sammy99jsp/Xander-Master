use super::prelude::*;

pub struct SerializeUnsizedImpl<'a> {
    pub span: Span,
    pub tr: &'a NamespaceTr,
}

impl<'a> Impl for SerializeUnsizedImpl<'a> {
    fn find(impls: &mut Implementations) -> &mut Option<TokenStream> {
        &mut impls.serialize_unsized
    }

    fn generate(self) -> TokenStream {
        let Self { tr, span } = self;

        let s = utils::ty_param("S");

        let serialize_unsized = rkyv::SerializeUnsized(s.clone())
            .with_last(|a| a.set_span(span));
        
        let fallible = &rkyv::Fallible;
        let writer = &rkyv::Writer;
        let sharing = &rkyv::Sharing;
        let allocator = &rkyv::Allocator;

        let utils_serialize_unsized = &krate::utils_serialize_unsized;

        quote! {
            #[automatically_derived]
            impl<#s> #serialize_unsized for dyn #tr + '_
            where
                #s: #fallible
                    + #writer
                    + #sharing
                    + #allocator + ?Sized,
                #s::Error: ::core::error::Error + Send + Sync + 'static,
            {
                fn serialize_unsized(
                    &self,
                    serializer: &mut #s,
                ) -> Result<usize, <#s as #fallible>::Error> {
                    unsafe { #utils_serialize_unsized(self, serializer) }
                }
            }
        }
        .respan(span)
    }
}
