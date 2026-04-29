use rkyv::{
    SerializeUnsized,
    rancor::Fallible,
    ser::{Allocator, Positional, Sharing, Writer, WriterExt, sharing::SharingState},
};

use crate::dynx::error::DynError;

pub trait DynSerializer: Positional {
    fn write(&mut self, bytes: &[u8]) -> Result<(), DynError>;

    /// # Safety
    /// Same as [Allocator::push_alloc]
    unsafe fn push_alloc(
        &mut self,
        layout: std::alloc::Layout,
    ) -> Result<std::ptr::NonNull<[u8]>, DynError>;

    /// # Safety
    /// Same as [Allocator::pop_alloc]
    unsafe fn pop_alloc(
        &mut self,
        ptr: std::ptr::NonNull<u8>,
        layout: std::alloc::Layout,
    ) -> Result<(), DynError>;

    fn start_sharing(&mut self, address: usize) -> SharingState;

    fn finish_sharing(&mut self, address: usize, pos: usize) -> Result<(), DynError>;
}

impl<'a> Fallible for dyn DynSerializer + 'a {
    type Error = DynError; // TODO: Fancier error type.
}

impl<'a> Writer for dyn DynSerializer + 'a {
    fn write(&mut self, bytes: &[u8]) -> Result<(), <Self as Fallible>::Error> {
        DynSerializer::write(self, bytes)
    }
}

unsafe impl<'a> Allocator for dyn DynSerializer + 'a {
    unsafe fn push_alloc(
        &mut self,
        layout: std::alloc::Layout,
    ) -> Result<std::ptr::NonNull<[u8]>, <Self as Fallible>::Error> {
        // SAFETY: Upheld by caller.
        unsafe { DynSerializer::push_alloc(self, layout) }
    }

    unsafe fn pop_alloc(
        &mut self,
        ptr: std::ptr::NonNull<u8>,
        layout: std::alloc::Layout,
    ) -> Result<(), <Self as Fallible>::Error> {
        // SAFETY: Upheld by caller.
        unsafe { DynSerializer::pop_alloc(self, ptr, layout) }
    }
}

impl<'a> Sharing for dyn DynSerializer + 'a {
    fn start_sharing(&mut self, address: usize) -> SharingState {
        DynSerializer::start_sharing(self, address)
    }

    fn finish_sharing(
        &mut self,
        address: usize,
        pos: usize,
    ) -> Result<(), <Self as Fallible>::Error> {
        DynSerializer::finish_sharing(self, address, pos)
    }
}

impl<E, S> DynSerializer for S
where
    E: Send + Sync + core::error::Error + 'static,
    S: Fallible<Error = E> + Writer + Allocator + Sharing,
{
    fn write(&mut self, bytes: &[u8]) -> Result<(), DynError> {
        <Self as Writer>::write(self, bytes).map_err(DynError::new)
    }

    unsafe fn push_alloc(
        &mut self,
        layout: std::alloc::Layout,
    ) -> Result<std::ptr::NonNull<[u8]>, DynError> {
        // SAFETY: Upheld by caller.
        unsafe { Allocator::push_alloc(self, layout).map_err(DynError::new) }
    }

    unsafe fn pop_alloc(
        &mut self,
        ptr: std::ptr::NonNull<u8>,
        layout: std::alloc::Layout,
    ) -> Result<(), DynError> {
        // SAFETY: Upheld by caller.
        unsafe { Allocator::pop_alloc(self, ptr, layout).map_err(DynError::new) }
    }

    fn start_sharing(&mut self, address: usize) -> SharingState {
        Sharing::start_sharing(self, address)
    }

    fn finish_sharing(&mut self, address: usize, pos: usize) -> Result<(), DynError> {
        Sharing::finish_sharing(self, address, pos).map_err(DynError::new)
    }
}

pub trait DynSerializeUnsized<'a> {
    fn serialize_unsized(&self, serializer: &'a mut dyn DynSerializer) -> Result<usize, DynError>;
}

impl<'a, T> DynSerializeUnsized<'a> for T
where
    T: SerializeUnsized<dyn DynSerializer + 'a>,
{
    fn serialize_unsized(&self, serializer: &'a mut dyn DynSerializer) -> Result<usize, DynError> {
        SerializeUnsized::serialize_unsized(self, serializer)
    }
}

// impl<'a, T> DynSerializeUnsized<'a> for T
// where
//     T: SerializeUnsized<dyn DynSerializer + 'a>,
// {
//     fn serialize_unsized(&self, serializer: &'a mut dyn DynSerializer) -> Result<usize, DynError> {
//         SerializeUnsized::serialize_unsized(self, serializer)
//     }
// }
