pub mod metadata;

use rapidhash::v3::{rapidhash_v3, rapidhash_v3_file};
use rkyv::ptr_meta;
use std::{
    collections::{BTreeMap, HashMap},
    sync::LazyLock,
};

pub use metadata::{
    Registered,
    archiving::{ArchivedLocalId, Archiving},
    deserializing::Deserializing,
    singleton::StoredSingleton,
};

use metadata::{Meta, Metadata};

pub trait Namespace {
    const ID: &'static str;
}

pub trait IntoNamespace {
    type Namespace: Namespace;
}

pub trait Identity: IdentityBase<<Self::Parent as IntoNamespace>::Namespace> {
    type Parent: IntoNamespace + ?Sized;
    const LOCAL_ID: &'static str;
}

pub trait IdentityBase<NS>
where
    NS: Namespace + ?Sized,
{
    fn local_id(&self) -> &'static str;
}

impl<I: Identity> IdentityBase<<I::Parent as IntoNamespace>::Namespace> for I {
    fn local_id(&self) -> &'static str {
        I::LOCAL_ID
    }
}

pub type HashTy = u32;

pub struct Registry {
    metadata: HashMap<HashTy, HashMap<HashTy, Record<Meta>>>,
    archived: HashMap<HashTy, HashMap<usize, HashTy>>,
}

fn crunch(raw: u64) -> HashTy {
    raw as HashTy
}

#[inline]
pub fn hash(s: &str) -> HashTy {
    crunch(rapidhash_v3(s.as_bytes()))
}

#[inline]
pub fn hash_file<R: std::io::Read>(source: R) -> std::io::Result<HashTy> {
    rapidhash_v3_file(source).map(crunch)
}

impl Registry {
    pub fn new() -> Self {
        Self {
            metadata: Default::default(),
            archived: Default::default(),
        }
    }

    pub fn metadata_entry(
        &mut self,
        namespace_hash: HashTy,
        local_hash: HashTy,
        default: impl FnOnce() -> Record<()>,
    ) -> &mut Record<Meta> {
        self.metadata
            .entry(namespace_hash)
            .or_default()
            .entry(local_hash)
            .or_insert_with(|| {
                let Record {
                    namespace_id,
                    local_id,
                    ..
                } = default();
                Record {
                    namespace_id,
                    local_id,
                    payload: Meta::default(),
                }
            })
    }

    pub fn archived_metadata_entry<Tr>(
        &mut self,
        archived_meta: ptr_meta::DynMetadata<Tr::Archived>,
    ) -> &mut HashTy
    where
        Tr: IntoNamespace + rkyv::ArchiveUnsized + ?Sized,
    {
        let archived_meta = unsafe {
            std::mem::transmute::<ptr_meta::DynMetadata<Tr::Archived>, usize>(archived_meta)
        };

        // if self.archived.contains_key(&archived_meta) {
        //     panic!("Two V-Tables for {} are the same!", Tr::Namespace::ID);
        // }

        self.archived
            .entry(hash(Tr::Namespace::ID))
            .or_default()
            .entry(archived_meta)
            .or_default()
    }

    fn hash_id(namespace_id: &str, local_id: &str) -> (HashTy, HashTy) {
        let namespace_hash = hash(namespace_id);
        let local_hash = hash(local_id);

        (namespace_hash, local_hash)
    }

    fn intern_and_hash(
        &mut self,
        namespace_id: &'static str,
        local_id: &'static str,
    ) -> (HashTy, HashTy) {
        let (namespace_hash, local_hash) = Self::hash_id(namespace_id, local_id);

        (namespace_hash, local_hash)
    }

    pub fn lookup_by_local<Tr>(&self, local: ArchivedLocalId) -> Option<&Meta>
    where
        Tr: IntoNamespace + ?Sized,
    {
        Some(
            &self
                .metadata
                .get(&hash(Tr::Namespace::ID))?
                .get(&local.as_u64())?
                .payload,
        )
    }

    pub fn lookup_by_hash(&self, namespace_hash: HashTy, local_hash: HashTy) -> Option<&Meta> {
        Some(
            &self
                .metadata
                .get(&namespace_hash)?
                .get(&local_hash)?
                .payload,
        )
    }

    pub fn enroll<M: Metadata>(&mut self, record: Record<M>) {
        let new @ Record {
            namespace_id,
            local_id,
            ..
        } = record;

        let (namespace_hash, local_hash) = self.intern_and_hash(namespace_id, local_id);

        self.metadata
            .entry(namespace_hash)
            .or_default()
            .entry(local_hash)
            .and_modify(|record| {
                M::inscribe(new, &mut record.payload);
            })
            .or_insert({
                let mut record = Record {
                    namespace_id,
                    local_id,
                    payload: Meta {
                        archiving: None,
                        deserializing: None,
                        stored_singleton: None,
                        extra: BTreeMap::new(),
                    },
                };

                M::inscribe(new, &mut record.payload);

                record
            });

        M::after_inscribe(record, self);
    }

    pub fn lookup<Tr>(&self, local_id: &str) -> Option<&Meta>
    where
        Tr: IntoNamespace + ?Sized,
    {
        let (namespace_hash, local_hash) = Self::hash_id(Tr::Namespace::ID, local_id);
        let record = self.metadata.get(&namespace_hash)?.get(&local_hash)?;

        Some(&record.payload)
    }

    pub fn lookup_by_archive<Tr>(
        &self,
        archived: ptr_meta::DynMetadata<Tr::Archived>,
    ) -> Option<&Meta>
    where
        Tr: rkyv::ArchiveUnsized + IntoNamespace + ?Sized,
    {
        // SAFETY: We are just using the vtable ptr here.
        let meta =
            unsafe { std::mem::transmute::<ptr_meta::DynMetadata<Tr::Archived>, usize>(archived) };
        let local_id = self.archived.get(&hash(Tr::Namespace::ID))?.get(&meta)?;

        let ns = self.metadata.get(&hash(Tr::Namespace::ID))?;

        Some(&ns.get(local_id)?.payload)
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

pub static REGISTRY: LazyLock<Registry> = LazyLock::new(|| {
    let mut registry = Registry::default();

    for record in inventory::iter::<Record<Archiving>>() {
        registry.enroll(*record);
    }

    for record in inventory::iter::<Record<Deserializing>>() {
        registry.enroll(*record);
    }

    for record in inventory::iter::<Record<StoredSingleton>>() {
        registry.enroll(*record);
    }

    for plugin in inventory::iter::<RegistryPlugin>() {
        (plugin.0)(&mut registry);
    }

    registry
});

#[derive(Debug, Clone, Copy)]
pub struct Record<Payload = Meta> {
    pub namespace_id: &'static str,
    pub local_id: &'static str,
    pub payload: Payload,
}

impl<Payload> Record<Payload> {
    pub const fn new<T, Tr>(payload: Payload) -> Self
    where
        T: Identity<Parent = Tr>,
        Tr: IntoNamespace + ?Sized,
    {
        Self {
            namespace_id: Tr::Namespace::ID,
            local_id: T::LOCAL_ID,
            payload,
        }
    }

    pub const fn new_manual(
        namespace_id: &'static str,
        local_id: &'static str,
        payload: Payload,
    ) -> Self {
        Self {
            namespace_id,
            local_id,
            payload,
        }
    }
}

pub struct RegistryPlugin(pub fn(&mut Registry));

inventory::collect!(RegistryPlugin);
