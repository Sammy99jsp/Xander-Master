use std::{
    io::Read,
    rc::{Rc, Weak},
};

use crate::{IntoNamespace, Namespace};

pub trait Identity: IdentityBase<<Self::Parent as IntoNamespace>::Namespace> {
    type Parent: IntoNamespace + ?Sized;
    const LOCAL_ID: &'static str;
}

#[derive(Debug, Clone, Copy)]
pub struct FullId {
    pub namespace_id: &'static str,
    pub local_id: &'static str,
}

pub trait IdentityBase<NS>
where
    NS: Namespace + ?Sized,
{
    fn local_id(&self) -> &'static str;
}

pub trait IdentityFull {
    const FULL_ID: FullId;
}

pub trait IdentityFullBase {
    fn full_id(&self) -> FullId;
}

impl FullId {
    pub const SEPARATOR: &'static str = "::";
    pub const fn new<I: Identity>() -> Self {
        FullId {
            namespace_id: <<I::Parent as IntoNamespace>::Namespace>::ID,
            local_id: I::LOCAL_ID,
        }
    }

    /// An Id without a namespace.
    pub const fn mononym(id: &'static str) -> Self {
        Self {
            namespace_id: "",
            local_id: id,
        }
    }
}

impl<I: Identity> IdentityBase<<I::Parent as IntoNamespace>::Namespace> for I {
    fn local_id(&self) -> &'static str {
        I::LOCAL_ID
    }
}

impl<I: IdentityFull> IdentityFullBase for I {
    fn full_id(&self) -> FullId {
        const { I::FULL_ID }
    }
}

impl<I> IdentityFull for I
where
    I: Identity,
{
    const FULL_ID: FullId = FullId::new::<I>();
}

impl<I> Identity for Box<I>
where
    I: Identity,
{
    type Parent = I::Parent;

    const LOCAL_ID: &'static str = I::LOCAL_ID;
}

impl<I> Identity for Rc<I>
where
    I: Identity,
{
    type Parent = I::Parent;

    const LOCAL_ID: &'static str = I::LOCAL_ID;
}

impl<I> Identity for Weak<I>
where
    I: Identity,
{
    type Parent = I::Parent;

    const LOCAL_ID: &'static str = I::LOCAL_ID;
}

impl Namespace for ! {
    const ID: &'static str = "NEVER";
}

impl IntoNamespace for ! {
    type Namespace = !;
}

impl Identity for ! {
    type Parent = !;

    const LOCAL_ID: &'static str = "NEVER";
}

// HASHING

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

impl FullId {
    pub fn hash(self) -> super::HashTy {
        // SAFETY: <FullIdReader as Read>::read will never return Err(_).
        unsafe { super::hash_file(FullIdReader::new(self)).unwrap_unchecked() }
    }
}
