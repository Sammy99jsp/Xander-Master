//! Handles Identity-based traits (Namespace, IntoNamespace, Identity)

use syn::spanned::Spanned;

use crate::namespace::parsing::NamespaceTy;

use super::prelude::*;

pub struct NamespaceImpl<'a> {
    pub span: Span,
    pub ns: NamespaceSt,
    pub id: Option<&'a syn::LitStr>,
}

impl<'a> From<&'a NamespaceTy> for NamespaceImpl<'a> {
    fn from(value: &'a NamespaceTy) -> Self {
        match value {
            NamespaceTy::New { id, ident, .. } => Self {
                span: ident.span(),
                ns: NamespaceSt(ident.clone().into()),
                id: Some(id),
            },
            NamespaceTy::Existing { path, .. } => Self {
                span: path.span(),
                ns: NamespaceSt(path.clone()),
                id: None,
            },
        }
    }
}

impl<'a> Impl for NamespaceImpl<'a> {
    fn find(impls: &mut Implementations) -> &mut Option<TokenStream> {
        &mut impls.namespace
    }

    fn generate(self) -> TokenStream {
        let Self { ns, id, span } = self;

        let namespace = &krate::Namespace;

        id.map(|id| {
            // let span = span.resolved_at(Span::call_site());
            let ns_no_span = ns.0.with_last(|l| l.set_span(Span::call_site()));
            let id_ident = syn::Ident::new("ID", id.span());

            quote! {
                pub struct #ns;
                #[automatically_derived]
                impl #namespace for #ns_no_span {
                    const #id_ident: &'static str = #id;
                }
            }
            .respan(span)
        })
        .unwrap_or_default()
    }
}

pub struct IntoNamespaceImpl<'a> {
    span: Span,
    pub ns: &'a NamespaceSt,
    pub tr: &'a NamespaceTr,
}

impl<'a> IntoNamespaceImpl<'a> {
    pub fn new(tr: &'a NamespaceTr, ns: &'a NamespaceSt) -> Self {
        Self {
            span: tr.span(),
            ns,
            tr,
        }
    }
}

impl<'a> Impl for IntoNamespaceImpl<'a> {
    fn find(impls: &mut Implementations) -> &mut Option<TokenStream> {
        &mut impls.into_namespace
    }

    fn generate(self) -> TokenStream {
        let Self { ns, tr, span } = self;

        let into_namespace = &krate::IntoNamespace;
        let tr = tr.erase_span();
        let ns = ns.erase_span();

        quote! {
            #[automatically_derived]
            impl #into_namespace for dyn #tr {
                type Namespace = #ns;
            }
        }
        .respan(span)
    }
}
