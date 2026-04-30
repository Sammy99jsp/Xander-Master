#[doc(hidden)]
pub use crate::register_deserialize;

/// # Deserialize
#[doc(hidden)]
#[macro_export]
macro_rules! register_deserialize {
    (@<$($g: ident),*> () $this: path) => {
        compile_error!("Cannot use register(Deserialize) without parent trait!");
    };
    (@<$($g: ident),*> () $this: path: $tr: ty) => {
        const _: () = {
            unsafe impl $crate::dynx::registry::Registered<$crate::dynx::registry::Deserializing>
                for $crate::dynx::rkyv::Archived<$this>
            {
            }

            ::inventory::submit! {
                $crate::dynx::registry::Record::new::<$this, $tr>({
                    $crate::dynx::registry::Deserializing::new::<$this, $tr>()
                })
            }
        };
    };
}
