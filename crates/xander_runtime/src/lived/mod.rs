pub mod cell;
pub mod list;
pub mod provided;

pub use cell::LivedCell;
pub use list::LivedList;
pub use provided::{ArchivedProvisoBase, Provided, Proviso, ProvisoBase};

use std::{
    ops::Deref,
    rc::{Rc, Weak},
};

pub trait LivedIdentity: Lived {
    #[doc(hidden)]
    fn full_id(&self) -> FullId;
}

pub trait Lived {
    fn is_alive(&self) -> bool;
}

pub trait LivedAndSerializable: LivedIdentity + for<'a> DynSerializeUnsized<'a> {}

impl<T> LivedAndSerializable for T where
    T: Lived + LivedIdentity + for<'a> DynSerializeUnsized<'a> + ?Sized
{
}

impl<L> Lived for Rc<L>
where
    L: Lived + ?Sized,
{
    fn is_alive(&self) -> bool {
        L::is_alive(self)
    }
}

impl<L> LivedIdentity for Rc<L>
where
    L: LivedIdentity + ?Sized + 'static,
{
    fn full_id(&self) -> FullId {
        L::full_id(self)
    }
}

impl<L> Lived for Box<L>
where
    L: Lived + ?Sized,
{
    fn is_alive(&self) -> bool {
        Lived::is_alive(Box::deref(self))
    }
}

impl<L> LivedIdentity for Box<L>
where
    L: LivedIdentity + ?Sized,
{
    fn full_id(&self) -> FullId {
        L::full_id(self)
    }
}

/// [Weak<L>] is alive if there are still strong references, and `<L>` itself is still alive.
impl<L> Lived for Weak<L>
where
    L: Lived + ?Sized + 'static,
{
    fn is_alive(&self) -> bool {
        self.upgrade().as_ref().is_some_and(Lived::is_alive)
    }
}

/// [Weak<L>] is alive if there are still strong references, and `<L>` itself is still alive.
impl<L> LivedIdentity for Weak<L>
where
    L: LivedIdentity + ?Sized + 'static,
{
    fn full_id(&self) -> FullId {
        self.upgrade().map(|l| l.full_id()).unwrap_or(NONE_ID)
    }
}

/// [DynWeak<L>] is alive if there are still strong references, and `<L>` itself is still alive.
impl<L> Lived for DynWeak<L>
where
    L: Lived + ?Sized + 'static,
{
    fn is_alive(&self) -> bool {
        self.upgrade().as_ref().is_some_and(Lived::is_alive)
    }
}
/// [Weak<L>] is alive if there are still strong references, and `<L>` itself is still alive.
impl<L> LivedIdentity for DynWeak<L>
where
    L: LivedIdentity + ?Sized + 'static,
{
    fn full_id(&self) -> FullId {
        self.upgrade().map(|l| l.full_id()).unwrap_or(NONE_ID)
    }
}

const NONE_ID: FullId = FullId {
    namespace_id: "LIVED",
    local_id: "NONE",
};

#[repr(transparent)]
#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct OptionalDependency<L>(Option<L>);

impl<L> OptionalDependency<L> {
    pub const fn new(dependency: Option<L>) -> Self {
        Self(dependency)
    }

    pub fn into_option(self) -> Option<L> {
        self.0
    }
}

impl<L> Default for OptionalDependency<L> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<L> From<OptionalDependency<L>> for Option<L> {
    fn from(value: OptionalDependency<L>) -> Self {
        value.0
    }
}

impl<L> Lived for OptionalDependency<L>
where
    L: Lived + 'static,
{
    fn is_alive(&self) -> bool {
        self.0.as_ref().is_none_or(Lived::is_alive)
    }
}

impl<L> LivedIdentity for OptionalDependency<L>
where
    L: LivedIdentity + 'static,
{
    fn full_id(&self) -> FullId {
        self.0.as_ref().map(|l| l.full_id()).unwrap_or(NONE_ID)
    }
}

impl<Tr> Lived for Single<Tr>
where
    Tr: Singleton + Lived + ?Sized,
{
    fn is_alive(&self) -> bool {
        <Tr as Lived>::is_alive(self)
    }
}

impl<Tr> LivedIdentity for Single<Tr>
where
    Tr: Singleton + Lived + ?Sized,
{
    fn full_id(&self) -> FullId {
        FullId {
            namespace_id: Tr::Namespace::ID,
            local_id: self.local_id(),
        }
    }
}

/// Handy macro for a type to always return `true` in [Lived::is_alive].
#[macro_export]
macro_rules! always_alive {
    ($ty: path) => {
        impl $crate::lived::Lived for $ty {
            fn is_alive(&self) -> bool {
                true
            }
        }
    };

    ($ty: path, $id: expr) => {
        impl $crate::lived::Lived for $ty {
            fn is_alive(&self) -> bool {
                true
            }
        }
    };
}

/// Handy macro for a type to delegate its lifespan to
/// one of its fields in [Lived::is_alive].
///
/// It is common to use fields like:
/// - [Weak<dyn Lived>] for direct dependency
/// - [OptionalDependency<Weak<dyn Lived>>] : [Some] => direct dependency; [None] => [always_alive]
#[macro_export]
macro_rules! dependently_alive {
    ($ty: path, $field: tt) => {
        impl $crate::lived::Lived for $ty {
            fn is_alive(&self) -> bool {
                $crate::lived::Lived::is_alive(&self.$field)
            }
        }
    };
}

pub use always_alive;
use bytecheck::CheckBytes;
pub use dependently_alive;

use dynx::{
    Identity, IntoNamespace, Namespace,
    dynx::{DynCheckBytes, DynDeserializer, DynSerializeUnsized, Single, Singleton, utils},
    registry::{
        self, ArchivedLocalId, Archiving, Deserializing, REGISTRY, Record, Registered,
        RegistryPlugin,
    },
};
use rkyv::{
    ArchiveUnsized, DeserializeUnsized, ptr_meta,
    rancor::Fallible,
    ser::{Allocator, Sharing, Writer},
    traits::ArchivePointee,
};

use crate::{DynWeak, dynx::FullId};

pub mod prelude {
    pub use super::{Lived, OptionalDependency, always_alive, dependently_alive};
    pub use std::sync::{Arc, Weak};
}

// Archiving

#[doc(hidden)]
pub struct NS;

impl Namespace for NS {
    const ID: &'static str = "LIVED";
}

impl IntoNamespace for dyn LivedAndSerializable {
    type Namespace = NS;
}

pub struct Living {
    lived_impl: ptr_meta::DynMetadata<dyn ArchivedLived>,
    deserializing: Deserializing,
    full_id: FullId,
    archiving: Archiving,
}

#[doc(hidden)]
pub struct LivedDeserializing {}

impl Living {
    pub const fn new<T>(full_id: FullId) -> Self
    where
        T: LivedAndSerializable + rkyv::Archive + 'static,
        <T as rkyv::Archive>::Archived:
            ArchivedLived + rkyv::DeserializeUnsized<T, dyn DynDeserializer>,
    {
        Self {
            lived_impl: {
                let (_, metadata) =
                    ptr_meta::to_raw_parts(std::ptr::dangling::<<T as rkyv::Archive>::Archived>()
                        as *const dyn ArchivedLived);
                metadata
            },
            full_id,
            archiving: Archiving::new::<T, dyn LivedAndSerializable>(),
            deserializing: Deserializing::new::<T, dyn LivedAndSerializable>(),
        }
    }

    pub const fn new_auto<T>() -> Self
    where
        T: LivedAndSerializable + rkyv::Archive + Identity + 'static,
        <T as rkyv::Archive>::Archived:
            ArchivedLived + rkyv::DeserializeUnsized<T, dyn DynDeserializer>,
    {
        const { Self::new::<T>(FullId::new::<T>()) }
    }
}

inventory::collect!(Living);

inventory::submit! {
    RegistryPlugin(|registry| {
        for record in inventory::iter::<Living> {
            // Insert the custom metadata.
            let full_id_hash = record.full_id.hash();
            let registered = registry.metadata_entry(registry::hash(NS::ID), full_id_hash, || { Record { namespace_id: NS::ID, local_id: record.full_id.local_id, payload: ()}});
            registered.payload.extra.insert(NS::ID, Box::new(record.lived_impl));
            registered.payload.archiving.replace(record.archiving);
            registered.payload.deserializing.replace(record.deserializing);

            *registry.archived_metadata_entry::<dyn LivedAndSerializable>(record.lived_impl) = full_id_hash;

        }
    })
}

pub trait ArchivedLived:
    rkyv::Portable
    + Registered<LivedDeserializing>
    + Registered<Archiving>
    + Registered<Deserializing>
    + for<'a> DynCheckBytes<'a>
{
}

impl<'a> ArchiveUnsized for dyn LivedAndSerializable + 'a {
    type Archived = dyn ArchivedLived + 'a;

    fn archived_metadata(&self) -> rkyv::ArchivedMetadata<Self> {
        ArchivedLocalId::new_raw(self.full_id().hash())
    }
}

impl<S> rkyv::SerializeUnsized<S> for dyn LivedAndSerializable + '_
where
    S: Fallible + Writer + Sharing + Allocator + ?Sized,
    S::Error: core::error::Error + Send + Sync + 'static,
{
    fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, S::Error> {
        unsafe { utils::serialize::serialize_unsized(self, serializer) }
    }
}

impl ArchivePointee for dyn ArchivedLived + '_ {
    type ArchivedMetadata = ArchivedLocalId;

    fn pointer_metadata(
        archived: &Self::ArchivedMetadata,
    ) -> <Self as ptr_meta::Pointee>::Metadata {
        let local_hash = archived.as_u64();

        let meta = REGISTRY
            .lookup_by_hash(registry::hash(NS::ID), local_hash)
            .unwrap();

        *meta
            .extra
            .get(NS::ID)
            .unwrap()
            .as_ref()
            .downcast_ref::<ptr_meta::DynMetadata<dyn ArchivedLived>>()
            .unwrap()
    }
}

unsafe impl ptr_meta::Pointee for dyn ArchivedLived + '_ {
    type Metadata = ptr_meta::DynMetadata<Self>;
}

unsafe impl ptr_meta::Pointee for dyn LivedAndSerializable + '_ {
    type Metadata = ptr_meta::DynMetadata<Self>;
}

impl rkyv::traits::LayoutRaw for dyn LivedAndSerializable + '_ {
    fn layout_raw(
        metadata: <Self as ptr_meta::Pointee>::Metadata,
    ) -> Result<std::alloc::Layout, std::alloc::LayoutError> {
        Ok(metadata.layout())
    }
}

impl<D> DeserializeUnsized<dyn LivedAndSerializable, D> for dyn ArchivedLived
where
    D: Fallible + ?Sized,
    D::Error: 'static,
{
    unsafe fn deserialize_unsized(
        &self,
        deserializer: &mut D,
        out: *mut dyn LivedAndSerializable,
    ) -> Result<(), <D as Fallible>::Error> {
        unsafe { dynx::dynx::utils::deserialize::deserialize_unsized(self, deserializer, out) }
    }

    fn deserialize_metadata(&self) -> <dyn LivedAndSerializable as ptr_meta::Pointee>::Metadata {
        dynx::dynx::utils::deserialize::deserialize_metadata::<dyn LivedAndSerializable>(self)
    }
}

unsafe impl<C> CheckBytes<C> for dyn ArchivedLived + '_
where
    C: Fallible + ?Sized,
    C::Error: core::error::Error + Send + Sync + 'static,
{
    unsafe fn check_bytes(
        value: *const Self,
        context: &mut C,
    ) -> Result<(), <C as Fallible>::Error> {
        unsafe { dynx::dynx::utils::check_bytes::check_bytes(value, context) }
    }
}

impl rkyv::traits::LayoutRaw for dyn ArchivedLived + '_ {
    fn layout_raw(
        metadata: <Self as ptr_meta::Pointee>::Metadata,
    ) -> Result<std::alloc::Layout, std::alloc::LayoutError> {
        Ok(metadata.layout())
    }
}
