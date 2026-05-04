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

use dynx::{
    Identity,
    dynx::{DynSerializeUnsized, Single, Singleton},
    registry::{
        Registered,
        identity::{FullId, IdentityFullBase},
    },
};

use crate::{DynWeak, lived::archiving::Living};

pub trait Lived {
    fn is_alive(&self) -> bool;
}

pub trait LivedSerializable:
    Lived + IdentityFullBase + for<'a> DynSerializeUnsized<'a> + Registered<Living>
{
}

impl<T> LivedSerializable for T where
    T: Lived + IdentityFullBase + Registered<Living> + for<'a> DynSerializeUnsized<'a> + ?Sized
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

impl<L> Lived for Box<L>
where
    L: Lived + ?Sized,
{
    fn is_alive(&self) -> bool {
        Lived::is_alive(Box::deref(self))
    }
}

impl Lived for ! {
    fn is_alive(&self) -> bool {
        unreachable!()
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

/// [DynWeak<L>] is alive if there are still strong references, and `<L>` itself is still alive.
impl<L> Lived for DynWeak<L>
where
    L: Lived + ?Sized + 'static,
{
    fn is_alive(&self) -> bool {
        self.upgrade().as_ref().is_some_and(Lived::is_alive)
    }
}

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

impl<I> IdentityFullBase for OptionalDependency<I>
where
    I: Identity,
{
    fn full_id(&self) -> FullId {
        FullId::new::<I>()
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

/// Handy macro for a type to always return `true` in [Lived::is_alive].
#[macro_export]
macro_rules! always_alive {
    ($ty: path $(=>)?) => {
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
    ($ty: path => $field: tt) => {
        impl $crate::lived::Lived for $ty {
            fn is_alive(&self) -> bool {
                $crate::lived::Lived::is_alive(&self.$field)
            }
        }
    };
    ($ty: path, $field: tt) => {
        $crate::dependently_alive!($ty => $field);
    };
}

#[doc(hidden)]
pub mod macros {
    pub use always_alive as always;
    pub use dependently_alive as dependent;

    #[allow(non_upper_case_globals)]
    pub const always: () = ();

    pub struct FieldName(Option<!>);

    #[allow(unused_variables)]
    pub fn dependent(dep: FieldName) {}
}

#[doc(hidden)]
pub use crate::register_lived;

/// # Lived
#[doc(hidden)]
#[macro_export]
macro_rules! register_lived {
    (@autocomplete $helper: ident) => {
        const _: () = {
            use $crate::lived::macros::*;

            #[allow(path_statements)]
            $helper;
        };
    };
    (@helper @<$($g: ident),*> ($helper: ident) $((($($extra: tt)*)))? $this: path) => {
        $crate::register_lived!(@autocomplete $helper);
        $crate::lived::macros::$helper!($this => $($($extra)*)?);
    };
    (@helper @<$($g: ident),*> ($helper: ident, $($helpers: ident),*) (($($extra: tt)*), $(($($extras: tt)*)),*) $this: path) => {
        $crate::register_lived!(@helper @<$($g: ident),*> ($helper) (($($extra)*)) $this);
        $crate::register_lived!(@helper @<$($g: ident),*> ($($helpers),*) ($(($($extras)*)),*) $this);
    };

    (@inner @<$($g: ident),*> () $this: path) => {};
    (@inner @<$($g: ident),*> (@) $this: path) => {
        unsafe impl<$($g),*> $crate::dynx::registry::Registered<$crate::dynx::registry::Deserializing> for rkyv::Archived<$this> {}
        unsafe impl<$($g),*> $crate::dynx::registry::Registered<$crate::dynx::registry::Archiving> for rkyv::Archived<$this> {}
    };
    (@inner @<$($g: ident),*> ($($f: ident $(($($tt: tt)*))?),+ $(,)?) $this: path) => {
        $crate::register_lived!(@inner @<$($g),*> () $this);
        $crate::register_lived!(@helper @<$($g),*> ($($f),*) ($(($($($tt)*)?)),*) $this);
    };
    (@inner @<$($g: ident),*> (@ $($f: ident $(($($tt: tt)*))?),+ $(,)?) $this: path) => {
        $crate::register_lived!(@inner @<$($g),*> (@) $this);
        $crate::register_lived!(@helper @<$($g),*> ($($f),*) ($(($($($tt)*)?)),*) $this);
    };
    (@<$($g: ident),*> ($($tt: tt)*) $this: path $(: $_tr: ty)?) => {
        impl<$($g),*> $crate::lived::archiving::ArchivedLived for ::rkyv::Archived<$this> {}

        unsafe impl<$($g),*> $crate::dynx::registry::Registered<$crate::lived::archiving::Living> for $this {}
        unsafe impl<$($g),*> $crate::dynx::registry::Registered<$crate::lived::archiving::LivedDeserializing> for rkyv::Archived<$this> {}

        $crate::register_lived!(@inner @<$($g),*> ($($tt)*) $this);

        ::inventory::submit! {
            $crate::lived::archiving::Living::new::<$this>()
        }
    };
}

pub mod prelude {
    pub use super::{Lived, OptionalDependency};
    pub use std::sync::{Arc, Weak};
}

// Archiving

pub mod archiving {
    use super::LivedSerializable;
    use bytecheck::CheckBytes;
    use dynx::{
        IntoNamespace, Namespace,
        dynx::{DynCheckBytes, DynDeserializer, utils},
        registry::{
            self, ArchivedLocalId, Archiving, Deserializing, REGISTRY, Record, Registered,
            RegistryPlugin,
            identity::{FullId, IdentityFull},
        },
    };
    use rkyv::{
        ArchiveUnsized, DeserializeUnsized, ptr_meta,
        rancor::Fallible,
        ser::{Allocator, Sharing, Writer},
        traits::ArchivePointee,
    };

    #[doc(hidden)]
    pub struct NS;

    impl Namespace for NS {
        const ID: &'static str = "LIVED";
    }

    impl IntoNamespace for dyn LivedSerializable {
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
        pub const fn new<T>() -> Self
        where
            T: LivedSerializable + rkyv::Archive + IdentityFull + 'static,
            <T as rkyv::Archive>::Archived:
                ArchivedLived + rkyv::DeserializeUnsized<T, dyn DynDeserializer>,
        {
            const {
                Self {
                    lived_impl: {
                        let (_, metadata) = ptr_meta::to_raw_parts(std::ptr::dangling::<
                            <T as rkyv::Archive>::Archived,
                        >()
                            as *const dyn ArchivedLived);
                        metadata
                    },
                    full_id: <T as IdentityFull>::FULL_ID,
                    archiving: Archiving::new::<T, dyn LivedSerializable>(),
                    deserializing: Deserializing::new::<T, dyn LivedSerializable>(),
                }
            }
        }
    }

    inventory::collect!(Living);

    inventory::submit! {
        RegistryPlugin(|registry| {
            for record in inventory::iter::<Living> {
                // Insert the custom metadata.
                let full_id_hash =  record.full_id.hash();
                let registered = registry.metadata_entry(registry::hash(NS::ID), full_id_hash, || { Record { namespace_id: NS::ID, local_id: record.full_id.local_id, payload: ()}});

                registered.payload.extra.insert(NS::ID, Box::new(record.lived_impl));
                registered.payload.archiving.replace(record.archiving);
                registered.payload.deserializing.replace(record.deserializing);

                *registry.archived_metadata_entry::<dyn LivedSerializable>(record.lived_impl) = full_id_hash;

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

    impl<'a> ArchiveUnsized for dyn LivedSerializable + 'a {
        type Archived = dyn ArchivedLived + 'a;

        fn archived_metadata(&self) -> rkyv::ArchivedMetadata<Self> {
            ArchivedLocalId::new_raw(self.full_id().hash())
        }
    }

    impl<S> rkyv::SerializeUnsized<S> for dyn LivedSerializable + '_
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
            let local_hash = archived.as_hash();

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

    unsafe impl ptr_meta::Pointee for dyn LivedSerializable + '_ {
        type Metadata = ptr_meta::DynMetadata<Self>;
    }

    impl rkyv::traits::LayoutRaw for dyn LivedSerializable + '_ {
        fn layout_raw(
            metadata: <Self as ptr_meta::Pointee>::Metadata,
        ) -> Result<std::alloc::Layout, std::alloc::LayoutError> {
            Ok(metadata.layout())
        }
    }

    impl<D> DeserializeUnsized<dyn LivedSerializable, D> for dyn ArchivedLived
    where
        D: Fallible + ?Sized,
        D::Error: 'static,
    {
        unsafe fn deserialize_unsized(
            &self,
            deserializer: &mut D,
            out: *mut dyn LivedSerializable,
        ) -> Result<(), <D as Fallible>::Error> {
            unsafe { dynx::dynx::utils::deserialize::deserialize_unsized(self, deserializer, out) }
        }

        fn deserialize_metadata(&self) -> <dyn LivedSerializable as ptr_meta::Pointee>::Metadata {
            dynx::dynx::utils::deserialize::deserialize_metadata::<dyn LivedSerializable>(self)
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
}
