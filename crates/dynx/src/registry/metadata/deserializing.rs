use std::marker::Unsize;

use rkyv::{ArchiveUnsized, Portable, traits::ArchivePointee};

use super::*;
use crate::{
    IntoNamespace,
    dynx::{DynDeserializer, DynError},
    rkyv::ptr_meta,
};

/// The necessary metadata for deserializing.
#[derive(Debug, Clone, Copy)]
pub struct Deserializing {
    #[doc(hidden)]
    pub erased_deserialize_fn: ErasedDeserializeUnsizedFn,
}

impl Metadata for Deserializing {
    fn inscribe(record: Record<Self>, meta: &mut Meta) {
        // if meta.archiving.is_none() {
        //     panic!(
        //         "Expected {}::{} to have a rkyv::Archive implementation!",
        //         record.namespace_id, record.local_id
        //     );
        // }

        meta.deserializing.replace(record.payload);
    }
}

impl Deserializing {
    pub const fn new<T, Tr>() -> Self
    where
        Tr: IntoNamespace + rkyv::ArchiveUnsized + ?Sized,
        T: rkyv::Archive,
        T: Unsize<Tr>,
        Tr: ptr_meta::Pointee<Metadata = ptr_meta::DynMetadata<Tr>>,
        Tr::Archived: ptr_meta::Pointee<Metadata = ptr_meta::DynMetadata<Tr::Archived>>,
        T::Archived: rkyv::DeserializeUnsized<T, dyn DynDeserializer>,
        <T as rkyv::Archive>::Archived: Unsize<<Tr as rkyv::ArchiveUnsized>::Archived>,
    {
        Self {
            erased_deserialize_fn: erased_deserialize_unsized::<T::Archived, Tr::Archived, T, Tr>(),
        }
    }
}

type ErasedDeserializeUnsizedFn = unsafe fn(
    *const (),
    deserializer: *mut dyn DynDeserializer,
    out: *mut (),
) -> Result<(), DynError>;

#[doc(hidden)]
pub const fn erased_deserialize_unsized<A, Ar, T, Tr>() -> ErasedDeserializeUnsizedFn
where
    A: rkyv::DeserializeUnsized<T, dyn DynDeserializer> + Unsize<Ar>,
    T: Unsize<Tr>,
    Tr: ArchiveUnsized<Archived = Ar> + ?Sized,
    Ar: ArchivePointee + Portable + ?Sized,
{
    |ar, deserializer, out| {
        // SAFETY: It is secretly a &A through type erasure.
        let ar = unsafe { (ar as *const A).as_ref().unwrap() };

        let out = out as *mut T;

        // SAFETY: since we are literally just passing arguments through, all safety invariants should be upheld.
        //         Additionally, since rkyv already knows the metadata, *out will have a valid layout for T.
        unsafe {
            rkyv::DeserializeUnsized::deserialize_unsized(ar, deserializer.as_mut().unwrap(), out)
        }
    }
}

inventory::collect!(Record<Deserializing>);
