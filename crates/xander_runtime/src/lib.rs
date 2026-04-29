#![feature(
    try_trait_v2,
    debug_closure_helpers,
    never_type,
    local_waker,
    ptr_metadata
)]

pub mod dynx;
pub mod flow;
pub mod lived;
pub mod ui;
#[doc(hidden)]
pub mod __macros;

pub use futures;

pub use lived::Lived;

pub use dynx::weak::DynWeak;

#[macro_export]
macro_rules! register {
    (@autocomplete $register: ident $($($v: ident),+)?) => {
        const _: () = {
            const _: $crate::__macros::__register__autocomplete::AutocompleteFn = $crate::__macros::__register__autocomplete::$register;
            $(
                #[allow(unused_imports)]
                use $crate::__macros::__register__autocomplete::{$($v),*};
            )?
        };
    };
    (@inner @inner Lived $p: path) => {
        unsafe impl $crate::dynx::registry::Registered<::dynx::registry::Deserializing> for rkyv::Archived<$p> {}
        unsafe impl $crate::dynx::registry::Registered<::dynx::registry::Archiving> for rkyv::Archived<$p> {}
    };
    (@inner @inner Lived $p: path : $_tr: ty) => {};
    (@inner Lived $p: path $(: $_tr: ty)?) => {
        impl $crate::lived::ArchivedLived for ::rkyv::Archived<$p> {}

        unsafe impl $crate::dynx::registry::Registered<$crate::lived::Living> for $p {}
        unsafe impl $crate::dynx::registry::Registered<$crate::lived::LivedDeserializing> for rkyv::Archived<$p> {}
        register!(@inner @inner Lived $p $(: $_tr)?);

        impl $crate::lived::LivedIdentity for $p {
            fn full_id(&self) -> $crate::dynx::FullId {
                $crate::dynx::FullId::new::<$p>()
            }
        }

        ::inventory::submit! {
            $crate::lived::Living::new_auto::<$p>()
        }
    };
    (@inner Lived($local_id: expr) $p: path $(: $_tr: ty)?) => {
        impl $crate::lived::ArchivedLived for ::rkyv::Archived<$p> {}

        unsafe impl $crate::dynx::registry::Registered<$crate::lived::Living> for $p {}
        unsafe impl $crate::dynx::registry::Registered<$crate::lived::LivedDeserializing> for rkyv::Archived<$p> {}

        register!(@inner @inner Lived $p $(: $_tr)?);

        impl $crate::lived::LivedIdentity for $p {
            fn full_id(&self) -> $crate::dynx::FullId {
                $crate::dynx::FullId::mononym($local_id)
            }
        }

        ::inventory::submit! {
            $crate::lived::Living::new::<$p>($crate::dynx::FullId::mononym($local_id))
        }
    };
    (@inner Archive $p: path) => {
        compile_error!("Cannot use register(Archive) without parent trait!");
    };
    (@inner Deserialize $p: path) => {
        compile_error!("Cannot use register(Deserialize) without parent trait!");
    };
    (@inner $v: ident $p: path: $tr: ty) => {
        const _: () = {
            #[allow(unused_imports)]
            use $crate::__macros::__register__autocomplete::{Archive, Deserialize, Singleton, Lived};

            unsafe impl $crate::dynx::registry::Registered<$v>
                for $crate::dynx::rkyv::Archived<$p>
            {
            }

            ::inventory::submit! {
                $crate::dynx::registry::Record::new::<$p, $tr>({

                    $v::new::<$p, $tr>()
                })
            }
        };
    };
    ($p: path, $register: ident $(($($v: ident $(($($tt: tt)*))?),+ $(,)?))?) => {
        register!(@autocomplete $register $($($v),+)?);
        $($(register!(@inner $v $(($($tt)*))? $p);)*)?
    };
    ($p: path: $tr: ty, $register: ident $(($($v: ident $(($($tt: tt)*))?),+ $(,)?))?) => {
        $(
            register!(@autocomplete $register $($v),+);
        )?
        $($(register!(@inner $v $(($($tt)*))? $p :$tr);)*)?
    };
}
