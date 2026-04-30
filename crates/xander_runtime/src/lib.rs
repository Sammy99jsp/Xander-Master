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
#[doc(hidden)]
pub mod macros;
pub mod ui;

pub use futures;

pub use lived::Lived;

pub use dynx::weak::DynWeak;

#[macro_export]
macro_rules! register {
    (@autocomplete $register: ident $($($v: ident),+)?) => {
        const _: () = {
            const _: $crate::macros::AutocompleteFn = $crate::macros::$register;
            $(
                #[allow(unused_imports)]
                use $crate::macros::*;

                $(
                    #[allow(path_statements)]
                    const _: () = {$v;};
                )*
            )?
        };
    };
    // register!(@autocomplete $register $($v),+);
    ($p: path $(:$tr: ty)?, $register: ident) => {
        register!(@autocomplete $register);
        compile_error!("Ensure you call register with parentheses `register!(..., register())`")
    };

    ($p: path $(:$tr: ty)?, $register: ident()) => {
        register!(@autocomplete $register);
    };

    (@inner @<$($g: ident),*> ($p: path $(:$tr: ty)?) ($v: ident) (($($extra: tt)*))) => {
        $crate::macros::$v!(@<$($g),*> ($($extra)*) $p $(:$tr)?);
    };
    (@inner @<$($g: ident),*> ($p: path $(:$tr: ty)?) ($v: ident, $($vs: ident),*) (($($extra: tt)*), $(($($extras: tt)*)),*)) => {
        $crate::register!(
            @inner @<$($g),*> ($p $(:$tr)?) ($v) (($($extra)*))
        );

        $crate::register!(
            @inner @<$($g),*> ($p $(:$tr)?) ($($vs),*) ($(($($extras)*)),*)
        );
    };

    ($(@<$($g: ident),*$(,)?>)? $p: path $(:$tr: ty)?, $register: ident ($($v: ident $(($($extra: tt)*))?),* $(,)?)) => {
        $crate::register!(@autocomplete $register $($v),*);

        $crate::register!( @inner
            @<$($($g),*)?> ($p $(:$tr)?) ($($v),*) ($(($($($extra)*)?)),*)
        );

    };

}
