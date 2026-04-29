pub mod byte_check;
pub mod de;
pub mod error;
pub mod ser;
pub mod singleton;
pub mod utils;

pub use self::{
    byte_check::{DynByteChecker, DynCheckBytes},
    de::DynDeserializer,
    error::DynError,
    ser::{DynSerializeUnsized, DynSerializer},
    singleton::{Single, Singleton},
};

use rkyv::ArchiveUnsized;
use std::marker::Unsize;

use crate::{
    Identity, IntoNamespace, Namespace,
    registry::{Deserializing, Registered},
};

#[doc(hidden)]
pub trait SerializesAs<P: Namespace + ?Sized> {}

impl<S> SerializesAs<<S::Parent as IntoNamespace>::Namespace> for S
where
    S: Identity + rkyv::SerializeUnsized<dyn DynSerializer>,
    S::Parent: ArchiveUnsized,
    S::Archived: Unsize<<S::Parent as ArchiveUnsized>::Archived>,
{
}

#[doc(hidden)]
pub trait DeserializesAs<P: Namespace + ?Sized> {}

impl<S> DeserializesAs<<S::Parent as IntoNamespace>::Namespace> for S
where
    S: Identity + rkyv::ArchiveUnsized,
    S::Parent: ArchiveUnsized,
    S::Archived: rkyv::DeserializeUnsized<Self, dyn DynDeserializer>
        + Unsize<<S::Parent as ArchiveUnsized>::Archived>
        + Registered<Deserializing>,
{
}

#[doc(hidden)]
pub trait ByteChecksAs<P: Namespace + ?Sized> {}

impl<S> ByteChecksAs<<S::Parent as IntoNamespace>::Namespace> for S
where
    S: Identity + rkyv::ArchiveUnsized,
    S::Parent: ArchiveUnsized,
    S::Archived: Unsize<<S::Parent as ArchiveUnsized>::Archived> + for<'a> DynCheckBytes<'a>,
{
}
