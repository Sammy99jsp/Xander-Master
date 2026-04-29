use super::prelude::*;

pub struct SingletonImpl<'a> {
    pub tr: &'a NamespaceTr,
    pub span: Span,
}

impl<'a> Impl for SingletonImpl<'a> {
    fn find(impls: &mut Implementations) -> &mut Option<TokenStream> {
        &mut impls.singleton_tr
    }

    fn generate(self) -> TokenStream {
        let Self { tr, span } = self;

        let singleton = &krate::Singleton.with_last(|p| p.set_span(span));

        quote! {
            impl #singleton for dyn #tr {}
        }
        .respan(span)
    }
}
