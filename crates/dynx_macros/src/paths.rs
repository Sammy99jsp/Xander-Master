use proc_macro2::Span;

macro_rules! path_def {
    (@generic_kind $_: ident) => {
        syn::Type
    };
    (@generic_kind $_: lifetime) => {
        syn::Lifetime
    };
    (@generic $_lt: lifetime @ $param: ident) => {
        let $param = syn::GenericArgument::Lifetime($param);
    };
    (@generic $_ty: ident @ $param: ident) => {
        let $param = syn::GenericArgument::Type($param);
    };
    (@inner $path: ident $i: ident <$($arg: ident),*>) => {
        $path.segments.push(
            syn::PathSegment {
                ident: ::syn::Ident::new(stringify!($i), proc_macro2::Span::call_site()),
                arguments: syn::PathArguments::AngleBracketed(
                    syn::AngleBracketedGenericArguments {
                        colon2_token: None,
                        lt_token: Default::default(),
                        args: syn::punctuated::Punctuated::from_iter([
                            $($arg),*
                        ]),
                        gt_token: Default::default()
                    }
                ),
            }
        );
    };
    (@inner $path: ident $i: ident) => {
        $path.segments.push(::syn::PathSegment::from(::syn::Ident::new(stringify!($i), proc_macro2::Span::call_site())));
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
            $v: vis fn $path_id: ident ($($gen: tt @ $param: ident),*) = $($t: tt $($i: ident)? $(<$({$arg : ident}),*>)?)::+;
        )*
    } => {
        $(
            $(#[$($attr)*])*
            #[allow(non_snake_case)]
            #[deny(unused_variables)]
            $v fn $path_id($($param: path_def!(@generic_kind $gen)),*) -> syn::Path {
            let mut path = ::syn::Path {
                leading_colon: Some(syn::token::PathSep::default()),
                segments: syn::punctuated::Punctuated::default(),
            };

            $(
                path_def!(@generic $gen @ $param);
            )*

            $(
                path_def!(@inner path $t $($i)? $(<$($arg),*>)?);
            )*

            path
        })*
    };
}

path_def! {
    /// library crate
    pub fn krate() = dynx;

    /// rkyv crate.
    fn rkyv() = $krate::rkyv;
}

pub mod rkyv {
    use super::rkyv;

    path_def! {
        pub fn ptr_meta() = $rkyv::ptr_meta;
        pub fn Pointee() = $ptr_meta::Pointee;
        pub fn DynMetadata(T @ t) = $ptr_meta::DynMetadata<{t}>;

        pub fn bytecheck() = $rkyv::bytecheck;
        pub fn CheckBytes() = $bytecheck::CheckBytes;

        pub fn Portable() = $rkyv::traits::Portable;
        pub fn LayoutRaw() = $rkyv::traits::LayoutRaw;
        pub fn ArchivePointee() = $rkyv::traits::ArchivePointee;
        pub fn Archived(T @ t) = $rkyv::Archived<{t}>;
        pub fn ArchivedMetadata(T @ t) = $rkyv::ArchivedMetadata<{t}>;
        pub fn ArchiveUnsized() = $rkyv::ArchiveUnsized;
        pub fn SerializeUnsized() = $rkyv::SerializeUnsized;
        pub fn DeserializeUnsized() = $rkyv::DeserializeUnsized;

        pub fn Fallible() = $rkyv::rancor::Fallible;
        pub fn Writer() = $rkyv::ser::Writer;
        pub fn Sharing() = $rkyv::ser::Sharing;
        pub fn Allocator() = $rkyv::ser::Allocator;
    }
}

pub mod inventory {
    path_def! {
        pub fn submit() = inventory::submit;
    }
}

pub mod core {
    path_def! {
        pub fn Error() = core::error::Error;
    }
}

pub mod std {
    path_def! {
        pub fn Layout() = std::alloc::Layout;
        pub fn LayoutError() = std::alloc::LayoutError;
    }
}

pub mod krate {
    use super::krate;

    path_def! {
        pub fn derive() = $krate::macros::derive;
        pub fn register() = $krate::macros::register;

        pub fn Namespace() = $krate::Namespace;
        pub fn IntoNamespace() = $krate::IntoNamespace;
        pub fn Identity() = $krate::Identity;
        pub fn IdentityBase() = $krate::registry::IdentityBase;
        pub fn ArchivedLocalId() = $krate::registry::ArchivedLocalId;

        pub fn SerializesAs(T @ t) = $krate::dynx::SerializesAs<{t}>;
        pub fn DeserializesAs(T @ t) = $krate::dynx::DeserializesAs<{t}>;

        pub fn Singleton() = $krate::dynx::Singleton;
        pub fn DynSerializer() = $krate::dynx::DynSerializer;
        pub fn DynDeserializer() = $krate::dynx::DynDeserializer;
        pub fn DynSerializeUnsized() = $krate::dynx::DynSerializeUnsized;
        pub fn DynCheckBytes() = $krate::dynx::DynCheckBytes;
        pub fn DynByteChecker() = $krate::dynx::DynByteChecker;
        pub fn DynError() = $krate::dynx::DynError;

        pub fn Record() = $krate::registry::Record;

        pub fn Registered(T @ t) = $krate::registry::Registered<{t}>;
        pub fn Archiving() = $krate::registry::Archiving;
        pub fn Deserializing() = $krate::registry::Deserializing;
        pub fn StoredSingleton() = $krate::registry::StoredSingleton;

        pub fn REGISTRY() = $krate::registry::REGISTRY;
    }
}

pub fn self_ty() -> syn::Type {
    ty_for(syn::Ident::new("Self", Span::call_site()).into())
}

pub fn ty_for(path: syn::Path) -> syn::Type {
    syn::Type::Path(syn::TypePath { qself: None, path })
}
