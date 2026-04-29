use rkyv::rancor::Fallible;

use crate::dynx::DynError;

unsafe fn force_as_dyn_trait<T, Tr>(value: &mut T) -> &mut Tr
where
    Tr: ?Sized,
    Tr: core::ptr::Pointee<Metadata = core::ptr::DynMetadata<Tr>>,
    T: ?Sized,
{
    let (thin, metadata) = (value as *mut T).to_raw_parts();

    // Evil hack using compiler intrinsics to avoid T, Tr: 'static.
    // I hope this doesn't come back to bite me!
    let info = const {
        let t = core::intrinsics::type_id::<T>();
        let tr = core::intrinsics::type_id::<Tr>();

        if t != tr
            && let Some(info) = core::intrinsics::type_id_vtable(t, tr)
        {
            unsafe {
                Some(core::mem::transmute::<
                    core::ptr::DynMetadata<*const ()>,
                    core::ptr::DynMetadata<Tr>,
                >(info))
            }
        } else {
            None
        }
    };

    // SAFETY: If T: Sized + Tr, then the code goes here:
    if let Some(metadata) = info {
        let ptr = core::ptr::from_raw_parts_mut::<Tr>(thin, metadata);

        // SAFETY: Still valid mutable reference, since it was originally a mutable reference.
        return unsafe { ptr.as_mut().unwrap_unchecked() };
    }

    if typeid::of::<T>() != typeid::of::<Tr>() {
        panic!("We cannot deal with supertraits, or non-impl'ing types!");
    }

    // SAFETY:  The only way we are here is if:
    //              T == Tr
    let metadata = &metadata as *const <T as core::ptr::Pointee>::Metadata;
    let metadata = metadata as *const core::ptr::DynMetadata<Tr>;
    let metadata = unsafe { core::ptr::read(metadata) };

    let ptr = core::ptr::from_raw_parts_mut::<Tr>(thin, metadata);

    // SAFETY: Valid for same reason as in `if` statement.
    unsafe { ptr.as_mut().unwrap_unchecked() }
}

unsafe fn unbox_error<Ok, T, Tr>(result: Result<Ok, Tr::Error>) -> Result<Ok, T::Error>
where
    T: Fallible + ?Sized,
    T::Error: 'static,
    Tr: Fallible<Error = DynError> + ?Sized,
    Tr::Error: 'static,
{
    if const { core::any::TypeId::of::<T::Error>() }
        != const { core::any::TypeId::of::<DynError>() }
    {
        return result.map_err(|err| err.downcast::<T::Error>().unwrap());
    }

    // TODO: There's probably a cleverer way to do this that doesn't heap
    //       allocate with MaybeUninit or something.
    let result = Box::into_raw(Box::new(result)) as *mut Result<Ok, T::Error>;
    unsafe { *Box::from_raw(result) }
}

#[doc(hidden)]
pub mod deserialize {
    use rkyv::{ArchiveUnsized, rancor::Fallible};

    use crate::{
        IntoNamespace,
        dynx::DynDeserializer,
        registry::{Archiving, Deserializing, REGISTRY, Registered},
    };

    #[inline]
    pub unsafe fn deserialize_unsized<D, Tr>(
        archived: &Tr::Archived,
        deserializer: &mut D,
        out: *mut Tr,
    ) -> std::result::Result<(), D::Error>
    where
        D: Fallible + ?Sized,
        D::Error: 'static,
        Tr: IntoNamespace + ArchiveUnsized + ?Sized,
        Tr::Archived: Registered<Archiving>
            + Registered<Deserializing>
            + rkyv::ptr_meta::Pointee<Metadata = rkyv::ptr_meta::DynMetadata<Tr::Archived>>,
    {
        let (this, archive_meta) = rkyv::ptr_meta::to_raw_parts(archived);
        let record = REGISTRY
            .lookup_by_archive::<Tr>(archive_meta)
            .expect("To be registered!");

        let deserializer =
            unsafe { super::force_as_dyn_trait::<D, dyn DynDeserializer>(deserializer) };
        let (out, _) = out.to_raw_parts();

        let result = unsafe {
            (record.deserializing.unwrap().erased_deserialize_fn)(this, deserializer, out)
        };

        unsafe { super::unbox_error::<_, D, dyn DynDeserializer>(result) }
    }

    #[inline]
    pub fn deserialize_metadata<Tr>(
        archived: &Tr::Archived,
    ) -> <Tr as rkyv::ptr_meta::Pointee>::Metadata
    where
        Tr: ?Sized
            + ArchiveUnsized
            + IntoNamespace
            + rkyv::ptr_meta::Pointee<Metadata = rkyv::ptr_meta::DynMetadata<Tr>>,
        Tr::Archived: rkyv::ptr_meta::Pointee<Metadata = rkyv::ptr_meta::DynMetadata<Tr::Archived>>,
    {
        let (_, meta) = rkyv::ptr_meta::to_raw_parts(archived);
        let record = REGISTRY
            .lookup_by_archive::<Tr>(meta)
            .expect("To be registered!");

        unsafe { record.archiving.unwrap().cast::<Tr>().meta }
    }
}

#[doc(hidden)]
pub mod serialize {
    use rkyv::{
        rancor::Fallible,
        ser::{Allocator, Sharing, Writer, WriterExt},
    };

    use crate::{
        dynx::{DynSerializeUnsized, DynSerializer},
        registry::ArchivedLocalId,
    };

    pub unsafe fn serialize_unsized<Tr, S>(
        value: &Tr,
        serializer: &mut S,
    ) -> Result<usize, S::Error>
    where
        S: Fallible + Writer + Sharing + Allocator + ?Sized,
        S::Error: core::error::Error + Send + Sync + 'static,
        Tr: for<'a> DynSerializeUnsized<'a> + ?Sized,
    {
        let serializer =
            unsafe { super::force_as_dyn_trait::<S, dyn DynSerializer + 'static>(serializer) };

        let result = DynSerializeUnsized::serialize_unsized(value, serializer);

        unsafe { super::unbox_error::<_, S, dyn DynSerializer>(result) }
    }
}

#[doc(hidden)]
pub mod check_bytes {
    use rkyv::rancor::Fallible;

    use crate::dynx::{DynByteChecker, DynCheckBytes, utils::force_as_dyn_trait};

    pub unsafe fn check_bytes<Tr, C>(value: *const Tr, context: &mut C) -> Result<(), C::Error>
    where
        C: Fallible + ?Sized,
        C::Error: core::error::Error + Send + Sync + 'static,
        Tr: for<'a> DynCheckBytes<'a> + ?Sized,
    {
        let context = unsafe { force_as_dyn_trait::<C, dyn DynByteChecker>(context) };
        let result = unsafe { DynCheckBytes::check_bytes(value.as_ref().unwrap(), context) };

        unsafe { super::unbox_error::<_, C, dyn DynByteChecker>(result) }
    }
}
