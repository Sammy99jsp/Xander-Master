use quote::quote;

mod member;
mod namespace;
mod paths;

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
