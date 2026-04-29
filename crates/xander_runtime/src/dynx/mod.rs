pub mod weak;

use std::{io::Read, marker::PhantomData};

use ::dynx::registry::HashTy;
pub use ::dynx::*;

pub struct Id<P: ?Sized> {
    local_id: &'static str,
    _namespace: PhantomData<P>,
}

impl<P> Id<P>
where
    P: IntoNamespace + ?Sized,
{
    pub const fn id_for<T>() -> Self
    where
        T: Identity<Parent = P>,
    {
        Self {
            local_id: T::LOCAL_ID,
            _namespace: PhantomData,
        }
    }
}

impl<P> Copy for Id<P> where P: IntoNamespace + ?Sized {}

impl<P> Clone for Id<P>
where
    P: IntoNamespace + ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<P> std::fmt::Debug for Id<P>
where
    P: IntoNamespace + ?Sized,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", P::Namespace::ID, self.local_id)
    }
}

#[macro_export]
macro_rules! identity {
    ($(@<$($g: ident),*$(,)?>)? $this: path: $tr: ty, $id: expr) => {
        impl$(<$($g),*>)? $crate::dynx::Identity for $this {
            type Parent = $tr;
            const LOCAL_ID: &'static str = const { $id };
        }
    };
}

pub trait IdentityExt {
    fn full_id(&self) -> FullId;
}

impl<I> IdentityExt for I
where
    I: Identity,
{
    fn full_id(&self) -> FullId {
        FullId {
            namespace_id: <I::Parent as IntoNamespace>::Namespace::ID,
            local_id: I::LOCAL_ID,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FullId {
    pub namespace_id: &'static str,
    pub local_id: &'static str,
}

impl FullId {
    pub const SEPARATOR: &str = "::";

    pub const fn new<T>() -> Self
    where
        T: Identity,
    {
        Self {
            namespace_id: <T::Parent as IntoNamespace>::Namespace::ID,
            local_id: T::LOCAL_ID,
        }
    }

    pub const fn mononym(id: &'static str) -> Self {
        Self {
            namespace_id: "",
            local_id: id,
        }
    }

    pub fn hash(self) -> HashTy {
        registry::hash_file(FullIdReader::new(self)).unwrap()
    }
}

#[derive(Debug, Clone, Copy)]
enum FullIdReaderStatus {
    Namespace(&'static [u8]),
    Sep(&'static [u8]),
    Local(&'static [u8]),
}

pub struct FullIdReader {
    id: FullId,
    status: FullIdReaderStatus,
}

impl FullIdReader {
    pub fn new(id: FullId) -> Self {
        Self {
            id,
            status: FullIdReaderStatus::Namespace(id.namespace_id.as_bytes()),
        }
    }
}

impl Read for FullIdReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match &mut self.status {
            FullIdReaderStatus::Local([]) => Ok(0),
            FullIdReaderStatus::Local(left) => Read::read(left, buf),
            FullIdReaderStatus::Sep([]) => {
                let mut local = self.id.local_id.as_bytes();

                let written = Read::read(&mut local, buf)?;
                self.status = FullIdReaderStatus::Local(local);

                Ok(written)
            }
            FullIdReaderStatus::Sep(left) => Read::read(left, buf),
            FullIdReaderStatus::Namespace([]) => {
                let mut sep = FullId::SEPARATOR.as_bytes();

                let written = Read::read(&mut sep, buf)?;
                self.status = FullIdReaderStatus::Sep(sep);

                Ok(written)
            }
            FullIdReaderStatus::Namespace(left) => Read::read(left, buf),
        }
    }
}

impl std::fmt::Display for FullId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", &self.namespace_id, &self.local_id)
    }
}
