#[doc(hidden)]
pub use crate::register_archive;

/// # Archive
#[doc(hidden)]
#[macro_export]
macro_rules! register_archive {
    (@<$($g: ident),*> () $this: path) => {
        compile_error!("Cannot use register(Archive) without parent trait!");
    };
    (@<$($g: ident),*> () $this: path: $tr: ty) => {
        const _: () = {
            unsafe impl $crate::dynx::registry::Registered<$crate::dynx::registry::Archiving>
                for $crate::dynx::rkyv::Archived<$this>
            {
            }

            ::inventory::submit! {
                $crate::dynx::registry::Record::new::<$this, $tr>({
                    $crate::dynx::registry::Archiving::new::<$this, $tr>()
                })
            }
        };
    };
}
