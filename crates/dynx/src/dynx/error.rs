use std::any::Any;

trait ErrorInner: core::error::Error + Send + Sync + 'static {
    fn as_any(self: Box<Self>) -> Box<dyn Any>;
}

type DynErrorInner = Box<dyn ErrorInner>;

impl<T> ErrorInner for T
where
    T: core::error::Error + Send + Sync + 'static,
{
    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self as Box<dyn Any>
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct DynError(DynErrorInner);

impl std::fmt::Display for DynError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl rkyv::rancor::Trace for DynError {
    fn trace<R>(self, _: R) -> Self
    where
        R: core::fmt::Debug + core::fmt::Display + Send + Sync + 'static,
    {
        self
    }
}

impl core::error::Error for DynError {}

impl rkyv::rancor::Source for DynError {
    fn new<T>(source: T) -> Self
    where
        T: core::error::Error + Send + Sync + 'static,
    {
        Self::new(source)
    }
}

impl DynError {
    pub fn new<T: core::error::Error + Send + Sync + 'static>(value: T) -> Self {
        Self(Box::new(value) as DynErrorInner)
    }

    pub fn downcast<T: 'static>(self) -> Option<T> {
        self.0.as_any().downcast::<T>().ok().map(|a| *a)
    }
}
