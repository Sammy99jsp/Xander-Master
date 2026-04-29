use super::prelude::*;

pub struct DeriveHelper {
    pub span: Span,
}

impl Impl for DeriveHelper {
    fn find(impls: &mut Implementations) -> &mut Option<TokenStream> {
        &mut impls.derive_helper
    }

    fn generate(self) -> TokenStream {
        let Self { span } = self;

        let derive = krate::derive.with_last(|p| p.set_span(span));
        // NOTE: No .respan() here because we only care about the ident.
        quote! {
            #[allow(unused_imports)]
            const _: () = {
                use #derive;
            };
        }
    }
}
