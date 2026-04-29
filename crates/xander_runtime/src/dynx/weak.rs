use std::{num::NonZeroUsize, ptr::NonNull, rc::Rc};

use rkyv::{
    ArchiveUnsized, DeserializeUnsized,
    de::Pooling,
    ptr_meta,
    rancor::{Fallible, ResultExt, Source},
    traits::LayoutRaw,
};

#[derive(rkyv::Archive, rkyv::Serialize)]
pub struct DynWeak<T>(std::rc::Weak<T>)
where
    T: ?Sized;

impl<T> DynWeak<T>
where
    T: ?Sized,
{
    pub fn empty() -> Self
    where
        T: std::ptr::Pointee<Metadata = std::ptr::DynMetadata<T>>,
    {
        unsafe { Self(new_weak_tr()) }
    }

    pub fn new(weak: std::rc::Weak<T>) -> Self {
        Self(weak)
    }

    /// Same as [std::rc::Weak::as_ptr]
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.0.as_ptr()
    }

    /// Same as [std::rc::Weak::upgrade]
    pub fn upgrade(&self) -> Option<Rc<T>> {
        self.0.upgrade()
    }
}

impl<T: ?Sized> Clone for DynWeak<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: ?Sized> std::fmt::Debug for DynWeak<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(Weak)")
    }
}

impl<T> Default for DynWeak<T>
where
    T: ?Sized,
    T: std::ptr::Pointee<Metadata = std::ptr::DynMetadata<T>>,
{
    fn default() -> Self {
        Self::empty()
    }
}

// Very hacky and nasty way to create an empty Weak<T> for unsized T.
// The pointer contained within me has not got valid metadata.
//
// The std creates dangling Weak<T> this way; we add invalid metadata,
// which kinda breaks the rules a bit.
unsafe fn new_weak_tr<T>() -> std::rc::Weak<T>
where
    T: ?Sized + std::ptr::Pointee<Metadata = std::ptr::DynMetadata<T>>,
{
    let fake_metadata =
        unsafe { std::mem::transmute::<NonZeroUsize, T::Metadata>(NonZeroUsize::MAX) };
    let dangling = NonNull::<T>::from_raw_parts(
        NonNull::<()>::without_provenance(NonZeroUsize::MAX),
        fake_metadata,
    );

    let weak = unsafe { std::rc::Weak::from_raw(dangling.as_ptr()) };

    if cfg!(debug_assertions) {
        assert!(weak.upgrade().is_none());
    }

    weak
}

impl<T, D> rkyv::Deserialize<DynWeak<T>, D> for rkyv::Archived<DynWeak<T>>
where
    T: ArchiveUnsized + LayoutRaw + ptr_meta::Pointee + ?Sized + 'static,
    T::Archived: DeserializeUnsized<T, D>,
    <T as ptr_meta::Pointee>::Metadata: Into<rkyv::de::pooling::Metadata> + rkyv::de::FromMetadata,
    T: std::ptr::Pointee<Metadata = std::ptr::DynMetadata<T>>,
    D: Fallible + Pooling + ?Sized,
    D::Error: Source,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<DynWeak<T>, <D as Fallible>::Error> {
        Ok(match self.0.upgrade() {
            Some(r) => DynWeak(Rc::downgrade(
                &r.deserialize(deserializer)
                    .with_trace(|| "inside DynWeak")?,
            )),
            // SAFETY: This should be a dangling pointer that std can recognize.
            None => unsafe { DynWeak(new_weak_tr()) },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::DynWeak;

    #[test]
    fn serialize() {
        trait A {}

        let weak: DynWeak<dyn A> = DynWeak::empty();
        assert!(weak.upgrade().is_none());
    }
}
