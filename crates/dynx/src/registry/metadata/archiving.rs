use bytecheck::CheckBytes;
use rkyv::{primitive::ArchivedU32, traits::NoUndef};

use crate::registry::{self, HashTy, hash};

use super::*;

#[repr(C)]
#[derive(
    Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, rkyv::Portable, CheckBytes,
)]
#[bytecheck(crate = rkyv::bytecheck)]
pub struct ArchivedLocalId(ArchivedU32);
unsafe impl NoUndef for ArchivedLocalId {}

impl ArchivedLocalId {
    pub fn as_hash(self) -> HashTy {
        self.0.to_native()
    }

    pub fn new(local_id: &str) -> Self {
        Self(ArchivedU32::from_native(registry::hash(local_id)))
    }

    pub fn new_raw(hash: HashTy) -> Self {
        Self(ArchivedU32::from_native(hash))
    }
}

/// All the necessary metadata for archiving.
#[derive(Debug, Clone, Copy)]
pub struct Archiving<Tr = (), Ar = ()>
where
    Tr: ?Sized,
    Ar: ?Sized,
{
    #[doc(hidden)]
    pub meta: ptr_meta::DynMetadata<Tr>,

    #[doc(hidden)]
    pub archived: ptr_meta::DynMetadata<Ar>,
}

impl Metadata for Archiving {
    fn inscribe(record: Record<Self>, meta: &mut Meta) {
        meta.archiving.replace(record.payload);
    }

    fn after_inscribe(record: Record<Self>, registry: &mut Registry) {
        let Record {
            local_id,
            payload,
            namespace_id,
            ..
        } = record;
        let local_hash = registry::hash(local_id);

        // SAFETY: We are using the vtable address as a key.
        //         This is not recommended because the compiler can combine vtables,
        //         But this will probably work for now.
        let archived_vtable =
            unsafe { std::mem::transmute::<ptr_meta::DynMetadata<()>, usize>(payload.archived) };

        if let Some(previous) = registry
            .archived
            .entry(hash(namespace_id))
            .or_default()
            .insert(archived_vtable, local_hash)
        {
            panic!("Both {previous} and {local_id} share the same Archived type vtable!")
        }
    }
}

impl Archiving {
    pub const fn new<T, Tr>() -> Self
    where
        Tr: IntoNamespace + rkyv::ArchiveUnsized + ?Sized,
        T: rkyv::Archive,
        T: Unsize<Tr>,
        Tr: ptr_meta::Pointee<Metadata = ptr_meta::DynMetadata<Tr>>,
        Tr::Archived: ptr_meta::Pointee<Metadata = ptr_meta::DynMetadata<Tr::Archived>>,
        <T as rkyv::Archive>::Archived: Unsize<<Tr as rkyv::ArchiveUnsized>::Archived>,
    {
        // SAFETY: We are type-erasing here, but NS_ID && L_ID => <Tr, Ar>
        unsafe {
            Self {
                meta: std::mem::transmute::<ptr_meta::DynMetadata<Tr>, ptr_meta::DynMetadata<()>>(
                    metadata_for::<T, Tr>(),
                ),
                archived: std::mem::transmute::<
                    ptr_meta::DynMetadata<Tr::Archived>,
                    ptr_meta::DynMetadata<()>,
                >(metadata_for::<
                    <T as rkyv::Archive>::Archived,
                    Tr::Archived,
                >()),
            }
        }
    }

    /// # Safety
    /// Only call with the &lt;Tr&gt; you created this [Archiving] with.
    pub unsafe fn cast<Tr>(self) -> Archiving<Tr, Tr::Archived>
    where
        Tr: rkyv::ArchiveUnsized + ?Sized,
    {
        unsafe { std::mem::transmute::<Archiving, Archiving<Tr, Tr::Archived>>(self) }
    }
}

const fn metadata_for<T, Tr>() -> Tr::Metadata
where
    Tr: ptr_meta::Pointee + ?Sized,
    T: Unsize<Tr>,
{
    ptr_meta::to_raw_parts(std::ptr::null::<T>() as *const Tr).1
}

inventory::collect!(Record<Archiving>);
