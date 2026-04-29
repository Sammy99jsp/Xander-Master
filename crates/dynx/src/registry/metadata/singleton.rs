use crate::dynx::{Single, Singleton};

use super::*;

#[derive(Debug, Clone, Copy)]
pub struct StoredSingleton {
    #[doc(hidden)]
    pub ptr: *const (),

    #[doc(hidden)]
    pub metadata: ptr_meta::DynMetadata<()>,
}

impl Metadata for StoredSingleton {
    fn inscribe(record: Record<Self>, meta: &mut Meta) {
        meta.stored_singleton.replace(record.payload);
    }
}

unsafe impl Send for StoredSingleton {}
unsafe impl Sync for StoredSingleton {}

impl StoredSingleton {
    pub const fn new<Tr>(stored_singleton: &'static Tr) -> Self
    where
        Tr: Singleton + ?Sized,
    {
        let (ptr, metadata) = ptr_meta::to_raw_parts(stored_singleton);
        let metadata = unsafe {
            std::mem::transmute::<ptr_meta::DynMetadata<Tr>, ptr_meta::DynMetadata<()>>(metadata)
        };

        Self { ptr, metadata }
    }

    /// # Safety
    /// Call this with the &lt;Tr&gt; used when creating this singleton!
    pub const unsafe fn cast<Tr>(self) -> Single<Tr>
    where
        Tr: Singleton + ?Sized,
    {
        let metadata = unsafe {
            std::mem::transmute::<ptr_meta::DynMetadata<()>, ptr_meta::DynMetadata<Tr>>(
                self.metadata,
            )
        };
        unsafe {
            Single(
                ptr_meta::from_raw_parts::<Tr>(self.ptr, metadata)
                    .as_ref()
                    .unwrap(),
            )
        }
    }
}

inventory::collect!(Record<StoredSingleton>);
