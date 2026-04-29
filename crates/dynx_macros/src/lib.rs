#![feature(fn_traits, unboxed_closures)]

use quote::quote;

mod member;
mod namespace;
pub(crate) mod utils;

/// See documentation of re-export in the main crate [scratchy::Namespace].
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Namespace(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    if attr.is_empty() {
        return quote! {
            compile_error!("Cannot leave #[Namespace] empty!")
        }
        .into();
    }

    let args = syn::parse_macro_input!(attr as namespace::Args);
    let tr = syn::parse_macro_input!(item as syn::ItemTrait);

    let tr_impls = args.process_for(tr);

    tr_impls.into()
}

/// See documentation of re-export in the main crate.
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Member(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    if attr.is_empty() {
        return quote! {
            compile_error!("Cannot leave #[Member] empty!")
        }
        .into();
    }

    let impl_ = syn::parse_macro_input!(item as syn::ItemImpl);
    let args = syn::parse_macro_input!(attr as member::Args);

    let tr_impls = args.process_for(impl_);

    tr_impls.into()
}

#[proc_macro]
pub fn id_case(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = syn::parse_macro_input!(tokens as syn::Ident);
    let lit = syn::LitStr::new(
        &heck::AsShoutySnakeCase(ident.to_string()).to_string(),
        ident.span(),
    );

    quote::quote! {
        #lit
    }
    .into()
}
