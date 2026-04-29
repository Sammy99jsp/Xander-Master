use rkyv::{
    de::{ErasedPtr, Pooling, PoolingState},
    rancor::Fallible,
};

use crate::dynx::error::DynError;

pub trait DynDeserializer {
    /// [Pooling::start_pooling]
    fn start_pooling(&mut self, address: usize) -> PoolingState;

    /// # Safety
    /// Same invariants as [Pooling::finish_pooling]
    unsafe fn finish_pooling(
        &mut self,
        address: usize,
        ptr: ErasedPtr,
        drop: unsafe fn(ErasedPtr),
    ) -> Result<(), DynError>;
}

impl<D> DynDeserializer for D
where
    D: Fallible + Pooling + ?Sized,
    D::Error: core::error::Error + Send + Sync + 'static,
{
    fn start_pooling(&mut self, address: usize) -> PoolingState {
        Pooling::start_pooling(self, address)
    }

    unsafe fn finish_pooling(
        &mut self,
        address: usize,
        ptr: ErasedPtr,
        drop: unsafe fn(ErasedPtr),
    ) -> Result<(), DynError> {
        // SAFETY: Caller must uphold the same invariants as us!
        unsafe { Pooling::finish_pooling(self, address, ptr, drop).map_err(DynError::new) }
    }
}

impl Fallible for dyn DynDeserializer + '_ {
    type Error = DynError;
}

impl Pooling for dyn DynDeserializer + '_ {
    fn start_pooling(&mut self, address: usize) -> PoolingState {
        DynDeserializer::start_pooling(self, address)
    }

    unsafe fn finish_pooling(
        &mut self,
        address: usize,
        ptr: ErasedPtr,
        drop: unsafe fn(ErasedPtr),
    ) -> Result<(), <Self as Fallible>::Error> {
        // SAFETY: Upheld by caller
        unsafe { DynDeserializer::finish_pooling(self, address, ptr, drop) }
    }
}
