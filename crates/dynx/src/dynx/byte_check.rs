use bytecheck::CheckBytes;
use rkyv::{rancor::Fallible, validation::ArchiveContext};

use crate::dynx::error::DynError;

/// Extra methods necessary from [ArchiveContext] fro validation.
pub trait DynByteChecker {
    fn check_subtree_ptr(
        &mut self,
        ptr: *const u8,
        layout: &std::alloc::Layout,
    ) -> Result<(), DynError>;

    /// # Safety
    /// Same invariants as [ArchiveContext::push_subtree_range].
    unsafe fn push_subtree_range(
        &mut self,
        root: *const u8,
        end: *const u8,
    ) -> Result<std::ops::Range<usize>, DynError>;

    /// # Safety
    /// Same invariants as [ArchiveContext::pop_subtree_range].
    unsafe fn pop_subtree_range(&mut self, range: std::ops::Range<usize>) -> Result<(), DynError>;
}

impl<T> DynByteChecker for T
where
    T: Fallible + ArchiveContext + ?Sized,
    T::Error: core::error::Error + Send + Sync + 'static,
{
    fn check_subtree_ptr(
        &mut self,
        ptr: *const u8,
        layout: &std::alloc::Layout,
    ) -> Result<(), DynError> {
        <T as ArchiveContext>::check_subtree_ptr(self, ptr, layout).map_err(DynError::new)
    }

    unsafe fn push_subtree_range(
        &mut self,
        root: *const u8,
        end: *const u8,
    ) -> Result<std::ops::Range<usize>, DynError> {
        // SAFETY: All invariants for the method must be upheld by the caller.
        unsafe { <T as ArchiveContext>::push_subtree_range(self, root, end).map_err(DynError::new) }
    }

    unsafe fn pop_subtree_range(&mut self, range: std::ops::Range<usize>) -> Result<(), DynError> {
        // SAFETY: All invariants for the method must be upheld by the caller.
        unsafe { <T as ArchiveContext>::pop_subtree_range(self, range).map_err(DynError::new) }
    }
}

impl<'a> Fallible for dyn DynByteChecker + 'a {
    type Error = DynError;
}

pub trait DynCheckBytes<'a> {
    /// # Safety
    /// Same requirements as [bytecheck::CheckBytes::check_bytes]
    unsafe fn check_bytes(&self, context: &'a mut dyn DynByteChecker) -> Result<(), DynError>;
}

impl<'a, T> DynCheckBytes<'a> for T
where
    T: bytecheck::CheckBytes<dyn DynByteChecker + 'a> + ?Sized,
{
    unsafe fn check_bytes(&self, context: &'a mut dyn DynByteChecker) -> Result<(), DynError> {
        // SAFETY: Caller is required to uphold all the relevant requirements.
        unsafe { CheckBytes::check_bytes(self, context) }
    }
}

unsafe impl<'a> rkyv::validation::ArchiveContext for dyn DynByteChecker + 'a {
    fn check_subtree_ptr(
        &mut self,
        ptr: *const u8,
        layout: &std::alloc::Layout,
    ) -> Result<(), <Self as Fallible>::Error> {
        DynByteChecker::check_subtree_ptr(self, ptr, layout)
    }

    unsafe fn push_subtree_range(
        &mut self,
        root: *const u8,
        end: *const u8,
    ) -> Result<std::ops::Range<usize>, <Self as Fallible>::Error> {
        unsafe { DynByteChecker::push_subtree_range(self, root, end) }
    }

    unsafe fn pop_subtree_range(
        &mut self,
        range: std::ops::Range<usize>,
    ) -> Result<(), <Self as Fallible>::Error> {
        unsafe { DynByteChecker::pop_subtree_range(self, range) }
    }
}
