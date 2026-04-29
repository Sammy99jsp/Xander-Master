use proc_macro2::{Span, TokenStream};
use quote::ToTokens;

#[repr(transparent)]
pub struct Path(fn() -> syn::Path);

impl Clone for Path {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl From<Path> for syn::Path {
    fn from(val: Path) -> Self {
        val.0()
    }
}

impl quote::ToTokens for Path {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.0().to_tokens(tokens);
    }
}

pub trait IntoGenericArg: Sized {
    fn into_generic_arg(self) -> syn::GenericArgument;
}

impl IntoGenericArg for syn::Lifetime {
    fn into_generic_arg(self) -> syn::GenericArgument {
        syn::GenericArgument::Lifetime(self)
    }
}

impl IntoGenericArg for syn::Type {
    fn into_generic_arg(self) -> syn::GenericArgument {
        syn::GenericArgument::Type(self)
    }
}

macro_rules! variadic_impl {
    ($helper_fn: ident => $($t: ident),* $(,)?) => {
        #[doc(hidden)]
        #[allow(non_snake_case, unused)]
        fn $helper_fn<$($t: IntoGenericArg),*>(path: &Path, $($t: $t),*) -> syn::Path {
            let mut path = path.0();
            let args = &mut path.segments.last_mut().unwrap().arguments;

            if const { core::mem::size_of::<($($t),*)>() == 0 } {
                return path;
            }

            match args {
                syn::PathArguments::None => {
                    *args =
                        syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                            colon2_token: None,
                            lt_token: Default::default(),
                            args: Default::default(),
                            gt_token: Default::default(),
                        });
                }
                syn::PathArguments::AngleBracketed(_) => (),
                syn::PathArguments::Parenthesized(_) => unimplemented!(),
            }

            let args = match args {
                syn::PathArguments::AngleBracketed(args) => &mut args.args,
                _ => unreachable!(),
            };

            $(
                args.push($t.into_generic_arg());
            )*

            path
        }

        #[allow(non_snake_case)]
        impl<$($t: IntoGenericArg),*> std::ops::FnOnce<($($t,)*)> for Path {
            type Output = syn::Path;

            extern "rust-call" fn call_once(self, ($($t,)*): ($($t,)*)) -> Self::Output {
                $helper_fn(&self, $($t,)*)
            }
        }

        #[allow(non_snake_case)]
        impl<$($t: IntoGenericArg),*> std::ops::FnMut<($($t,)*)> for Path {
            extern "rust-call" fn call_mut(&mut self, ($($t,)*): ($($t,)*)) -> Self::Output {
                $helper_fn(self, $($t,)*)
            }
        }

        #[allow(non_snake_case)]
        impl<$($t: IntoGenericArg),*> std::ops::Fn<($($t,)*)> for Path {
            extern "rust-call" fn call(&self, ($($t,)*): ($($t,)*)) -> Self::Output {
                $helper_fn(self, $($t,)*)
            }
        }

    };
}

variadic_impl!(f0 => );
variadic_impl!(f1 => T1,);
variadic_impl!(f2 => T1, T2,);
variadic_impl!(f3 => T1, T2, T3,);
variadic_impl!(f4 => T1, T2, T3, T4,);
variadic_impl!(f5 => T1, T2, T3, T4, T5,);

macro_rules! path_def {
    (@inner $path: ident $i: ident) => {
        $path.segments.push(::syn::PathSegment{
            ident: syn::Ident::new(stringify!($i), ::proc_macro2::Span::call_site()),
            arguments: syn::PathArguments::None
        });
    };
    (@inner $path: ident $t: tt $i: ident) => {
        path_def!(@inner $path |$t| $i)
    };
    (@inner $path: ident $(|)$+ $i: ident) => {
        $path.segments.extend(self::$i().segments);
    };
    {
        $(
            $(#[$($attr: tt)*])*
            $v: vis static $path_id: ident = $($t: tt $($i: ident)?)::+;
        )*
    } => {
        $(
            $(#[$($attr)*])*
            #[allow(non_upper_case_globals)]
            $v static $path_id: $crate::utils::Path = $crate::utils::Path(|| {
                let mut path = ::syn::Path {
                    leading_colon: Some(::syn::token::PathSep::default()),
                    segments: ::syn::punctuated::Punctuated::default(),
                };

                $(
                    path_def!(@inner path $t $($i)?);
                )*

                path
            });
        )*
    };
}

pub mod paths {
    path_def! {
        pub static krate = dynx;
        pub static rkyv = $krate::rkyv;
    }

    pub mod rkyv {
        use super::rkyv;

        path_def! {
            pub static Portable = $rkyv::Portable;

            pub static Archived = $rkyv::Archived;
            pub static ArchiveUnsized = $rkyv::ArchiveUnsized;
            pub static ArchivedMetadata = $rkyv::ArchivedMetadata;

            pub static ArchivePointee = $rkyv::traits::ArchivePointee;
            pub static LayoutRaw = $rkyv::traits::LayoutRaw;

            pub static SerializeUnsized = $rkyv::SerializeUnsized;
            pub static DeserializeUnsized = $rkyv::DeserializeUnsized;

            pub static ptr_meta = $rkyv::ptr_meta;
            pub static Pointee = $ptr_meta::Pointee;
            pub static DynMetadata = $ptr_meta::DynMetadata;

            pub static Fallible = $rkyv::rancor::Fallible;

            pub static Writer = $rkyv::ser::Writer;
            pub static Sharing = $rkyv::ser::Sharing;
            pub static Allocator = $rkyv::ser::Allocator;

            pub static bytecheck = $rkyv::bytecheck;
            pub static CheckBytes = $bytecheck::CheckBytes;

        }
    }

    pub mod krate {
        use super::krate;

        path_def! {
            // Macro helpers
            pub static macros = $krate::macros;
            pub static register = $macros::register;
            pub static derive = $macros::derive;



            pub static Identity = $krate::Identity;
            pub static Namespace = $krate::Namespace;
            pub static IntoNamespace = $krate::IntoNamespace;

            pub static registry = $krate::registry;
            pub static ArchivedLocalId = $krate::registry::ArchivedLocalId;
            pub static IdentityBase = $krate::registry::IdentityBase;
            pub static Record = $krate::registry::Record;
            pub static Registered = $krate::registry::Registered;
            pub static Archiving = $krate::registry::Archiving;
            pub static Deserializing = $krate::registry::Deserializing;
            pub static StoredSingleton = $krate::registry::StoredSingleton;

            pub static REGISTRY = $registry::REGISTRY;


            pub static dynx = $krate::dynx;
            pub static utils_serialize_unsized = $dynx::utils::serialize::serialize_unsized;
            pub static utils_deserialize_unsized = $dynx::utils::deserialize::deserialize_unsized;
            pub static utils_deserialize_metadata = $dynx::utils::deserialize::deserialize_metadata;
            pub static utils_check_bytes = $dynx::utils::check_bytes::check_bytes;

            pub static Singleton = $dynx::Singleton;
            pub static DynSerializeUnsized = $dynx::DynSerializeUnsized;
            pub static DynCheckBytes = $dynx::DynCheckBytes;

            pub static SerializesAs = $dynx::SerializesAs;
            pub static DeserializesAs = $dynx::DeserializesAs;
        }
    }

    pub mod inventory {
        path_def! {
            pub static submit = inventory::submit;
        }
    }
}

pub fn self_ty() -> syn::Type {
    syn::Type::Path(syn::TypePath {
        qself: None,
        path: syn::Ident::new("Self", Span::call_site()).into(),
    })
}

pub fn ty_param(s: &'static str) -> syn::Type {
    syn::Path::from(syn::Ident::new(s, Span::call_site())).to_type()
}

pub trait Respan {
    fn respan(self, span: Span) -> TokenStream;
    fn erase_span(self) -> TokenStream;
}

impl<T: ToTokens> Respan for T {
    fn respan(self, span: Span) -> TokenStream {
        self.into_token_stream()
            .into_iter()
            .map(|mut tt| {
                if let proc_macro2::TokenTree::Group(group) = &mut tt
                    && group.delimiter() == proc_macro2::Delimiter::Brace
                {
                    group.set_span(span);
                };

                tt
            })
            .collect()
    }

    fn erase_span(self) -> TokenStream {
        self.into_token_stream()
            .into_iter()
            .map(|mut tt| {
                tt.set_span(Span::call_site());
                tt
            })
            .collect()
    }
}

pub trait PathExt {
    fn with_last<F: FnOnce(&mut syn::Ident)>(&self, func: F) -> syn::Path;

    fn to_type(&self) -> syn::Type;
}

impl PathExt for Path {
    fn with_last<F: FnOnce(&mut syn::Ident)>(&self, func: F) -> syn::Path {
        let mut path = self();
        func(&mut path.segments.last_mut().unwrap().ident);
        path
    }

    fn to_type(&self) -> syn::Type {
        syn::Type::Path(syn::TypePath {
            qself: None,
            path: self(),
        })
    }
}

impl PathExt for syn::Path {
    fn with_last<F: FnOnce(&mut syn::Ident)>(&self, func: F) -> syn::Path {
        let mut path = self.clone();
        func(&mut path.segments.last_mut().unwrap().ident);
        path
    }

    fn to_type(&self) -> syn::Type {
        syn::Type::Path(syn::TypePath {
            qself: None,
            path: self.clone(),
        })
    }
}
