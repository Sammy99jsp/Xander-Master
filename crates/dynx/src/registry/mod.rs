use rkyv::{
    ArchiveUnsized, Portable, ptr_meta,
    rancor::Source,
    traits::{ArchivePointee, NoUndef},
};
use std::{collections::HashMap, ffi::CStr, marker::Unsize, str::Utf8Error, sync::LazyLock};

use crate::dynx::{DynDeserializer, DynError, Single, Singleton};

pub trait Namespace {
    const ID: &'static str;
}

pub trait IntoNamespace {
    type Namespace: Namespace;
}

pub trait Identity:
    IdentityBase<<Self::Parent as IntoNamespace>::Namespace> + Unsize<Self::Parent>
{
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

pub struct Registry {
    metadata: HashMap<&'static str, HashMap<&'static str, Record<Meta>>>,
    archived: HashMap<usize, &'static str>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            metadata: Default::default(),
            archived: Default::default(),
        }
    }

    pub fn enroll_archiving(&mut self, record: Record<Archiving>) {
        let Record {
            namespace_id,
            local_id,
            payload,
            ..
        } = record;

        self.metadata
            .entry(namespace_id)
            .or_default()
            .entry(local_id)
            .and_modify(|record| {
                record.payload.archiving = Some(payload);
            })
            .or_insert(Record {
                namespace_id,
                local_id,
                payload: Meta {
                    archiving: Some(payload),
                    deserializing: None,
                    stored_singleton: None,
                },
            });

        // SAFETY: We are using the vtable address as a key.
        //         This is not recommended because the compiler can combine vtables,
        //         But this will probably work for now.
        let archived_vtable =
            unsafe { std::mem::transmute::<ptr_meta::DynMetadata<()>, usize>(payload.archived) };

        if let Some(previous) = self.archived.insert(archived_vtable, local_id) {
            panic!("Both {previous} and {local_id} share the same Archived type vtable!")
        }
    }

    pub fn lookup<Tr>(&self, local_id: &str) -> Option<Meta>
    where
        Tr: IntoNamespace + ?Sized,
    {
        let record = *self.metadata.get(Tr::Namespace::ID)?.get(local_id)?;

        Some(record.payload)
    }

    pub fn lookup_by_archive<Tr>(
        &self,
        archived: ptr_meta::DynMetadata<Tr::Archived>,
    ) -> Option<Meta>
    where
        Tr: rkyv::ArchiveUnsized + IntoNamespace + ?Sized,
    {
        // SAFETY: We are just using the vtable ptr here.
        let meta =
            unsafe { std::mem::transmute::<ptr_meta::DynMetadata<Tr::Archived>, usize>(archived) };
        let local_id = self.archived.get(&meta);

        self.lookup::<Tr>(local_id?)
    }

    fn enroll_deserialize(&mut self, record: Record<Deserializing>) {
        let Some(
            existing @ Record {
                payload: Meta {
                    archiving: Some(_), ..
                },
                ..
            },
        ) = self
            .metadata
            .get_mut(&record.namespace_id)
            .and_then(|ns| ns.get_mut(&record.local_id))
        else {
            panic!(
                "Expected {}::{} to have a rkyv::Archive implementation!",
                record.namespace_id, record.local_id
            )
        };

        let replaced = existing.payload.deserializing.replace(record.payload);

        if replaced.is_some() {
            panic!(
                "Multiple rkyv::Deserialize impls for {}::{} registered!",
                record.namespace_id, record.local_id
            )
        }
    }

    fn enroll_singleton(
        &mut self,
        Record {
            namespace_id,
            local_id,
            payload,
        }: Record<StoredSingleton>,
    ) {
        self.metadata
            .entry(namespace_id)
            .or_default()
            .entry(local_id)
            .and_modify(|existing| {
                existing.payload.stored_singleton = Some(payload);
            })
            .or_insert(Record {
                namespace_id,
                local_id,
                payload: Meta {
                    archiving: None,
                    deserializing: None,
                    stored_singleton: Some(payload),
                },
            });
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
        registry.enroll_archiving(*record);
    }

    for record in inventory::iter::<Record<Deserializing>>() {
        registry.enroll_deserialize(*record);
    }

    for record in inventory::iter::<Record<StoredSingleton>>() {
        registry.enroll_singleton(*record);
    }

    registry
});

#[derive(Debug, Clone, Copy)]
pub struct Meta {
    pub archiving: Option<Archiving>,
    pub deserializing: Option<Deserializing>,
    pub stored_singleton: Option<StoredSingleton>,
}

/// All the necessary metadata for archiving.
#[derive(Debug, Clone, Copy)]
pub struct Archiving<Tr = (), Ar = ()>
where
    Tr: ?Sized,
    Ar: ?Sized,
{
    #[doc(hidden)]
    pub meta: ptr_meta::DynMetadata<Tr>,

    #[doc(hidden)]
    pub archived: ptr_meta::DynMetadata<Ar>,
}

impl Archiving {
    pub const fn new<T, Tr>() -> Self
    where
        Tr: IntoNamespace + rkyv::ArchiveUnsized + ?Sized,
        T: Identity<Parent = Tr> + rkyv::Archive,
        Tr: ptr_meta::Pointee<Metadata = ptr_meta::DynMetadata<Tr>>,
        Tr::Archived: ptr_meta::Pointee<Metadata = ptr_meta::DynMetadata<Tr::Archived>>,
        <T as rkyv::Archive>::Archived: Unsize<<Tr as rkyv::ArchiveUnsized>::Archived>,
    {
        // SAFETY: We are type-erasing here, but NS_ID && L_ID => <Tr, Ar>
        unsafe {
            Self {
                meta: std::mem::transmute::<ptr_meta::DynMetadata<Tr>, ptr_meta::DynMetadata<()>>(
                    metadata_for::<T, Tr>(),
                ),
                archived: std::mem::transmute::<
                    ptr_meta::DynMetadata<Tr::Archived>,
                    ptr_meta::DynMetadata<()>,
                >(metadata_for::<
                    <T as rkyv::Archive>::Archived,
                    Tr::Archived,
                >()),
            }
        }
    }
}

impl Archiving {
    /// # Safety
    /// Only call with the &lt;Tr&gt; you created this [Archiving] with.
    pub unsafe fn cast<Tr>(self) -> Archiving<Tr, Tr::Archived>
    where
        Tr: rkyv::ArchiveUnsized + ?Sized,
    {
        unsafe { std::mem::transmute::<Archiving, Archiving<Tr, Tr::Archived>>(self) }
    }
}

/// The necessary metadata for deserializing.
#[derive(Debug, Clone, Copy)]
pub struct Deserializing {
    #[doc(hidden)]
    pub erased_deserialize_fn: ErasedDeserializeUnsizedFn,
}

impl Deserializing {
    pub const fn new<T, Tr>() -> Self
    where
        Tr: IntoNamespace + rkyv::ArchiveUnsized + ?Sized,
        T: Identity<Parent = Tr> + rkyv::Archive,
        Tr: ptr_meta::Pointee<Metadata = ptr_meta::DynMetadata<Tr>>,
        Tr::Archived: ptr_meta::Pointee<Metadata = ptr_meta::DynMetadata<Tr::Archived>>,
        T::Archived: rkyv::DeserializeUnsized<T, dyn DynDeserializer>,
        <T as rkyv::Archive>::Archived: Unsize<<Tr as rkyv::ArchiveUnsized>::Archived>,
    {
        Self {
            erased_deserialize_fn: erased_deserialize_unsized::<T::Archived, Tr::Archived, T, Tr>(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StoredSingleton {
    #[doc(hidden)]
    pub ptr: *const (),

    #[doc(hidden)]
    pub metadata: ptr_meta::DynMetadata<()>,
}

unsafe impl Send for StoredSingleton {}
unsafe impl Sync for StoredSingleton {}

impl StoredSingleton {
    pub const fn new<Tr>(stored_singleton: &'static Tr) -> Self
    where
        Tr: Singleton + ?Sized,
    {
        let (ptr, metadata) = ptr_meta::to_raw_parts(stored_singleton);
        let metadata = unsafe {
            std::mem::transmute::<ptr_meta::DynMetadata<Tr>, ptr_meta::DynMetadata<()>>(metadata)
        };

        Self { ptr, metadata }
    }

    /// # Safety
    /// Call this with the &lt;Tr&gt; used when creating this singleton!
    pub const unsafe fn cast<Tr>(self) -> Single<Tr>
    where
        Tr: Singleton + ?Sized,
    {
        let metadata = unsafe {
            std::mem::transmute::<ptr_meta::DynMetadata<()>, ptr_meta::DynMetadata<Tr>>(
                self.metadata,
            )
        };
        unsafe {
            Single(
                ptr_meta::from_raw_parts::<Tr>(self.ptr, metadata)
                    .as_ref()
                    .unwrap(),
            )
        }
    }
}

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
}

/// In most cases, DO NOT implement this trait yourself! This will be auto-impl'd
/// by the [macro@crate::Namespace] macro.
///
/// # Safety
/// - For [Deserializing]: the [rkyv::Deserialize] implementation for this type must be present in the [Record] in the global [Registry].
/// - For [Archiving]: the [rkyv::traits::ArchivePointee] implementation for this type must be present in the [Record] in the global [Registry].
#[diagnostic::on_unimplemented(
    message = "{T} for {Self} is not recorded in the global Registry.",
    note = "Use the `#[Namespace(.., register(Archive, Deserialize))]` macro to register this implementation."
)]
pub unsafe trait Registered<T> {}

const fn metadata_for<T, Tr>() -> Tr::Metadata
where
    Tr: ptr_meta::Pointee + ?Sized,
    T: Unsize<Tr>,
{
    ptr_meta::to_raw_parts(std::ptr::null::<T>() as *const Tr).1
}

type ErasedDeserializeUnsizedFn = unsafe fn(
    *const (),
    deserializer: *mut dyn DynDeserializer,
    out: *mut (),
) -> Result<(), DynError>;

const fn erased_deserialize_unsized<A, Ar, T, Tr>() -> ErasedDeserializeUnsizedFn
where
    A: rkyv::DeserializeUnsized<T, dyn DynDeserializer> + Unsize<Ar>,
    T: Unsize<Tr>,
    Tr: ArchiveUnsized<Archived = Ar> + ?Sized,
    Ar: ArchivePointee + Portable + ?Sized,
{
    |ar, deserializer, out| {
        // SAFETY: It is secretly a &A through type erasure.
        let ar = unsafe { (ar as *const A).as_ref().unwrap() };

        let out = out as *mut T;

        // SAFETY: since we are literally just passing arguments through, all safety invariants should be upheld.
        //         Additionally, since rkyv already knows the metadata, *out will have a valid layout for T.
        unsafe {
            rkyv::DeserializeUnsized::deserialize_unsized(ar, deserializer.as_mut().unwrap(), out)
        }
    }
}

inventory::collect!(Record<Archiving>);
inventory::collect!(Record<Deserializing>);
inventory::collect!(Record<StoredSingleton>);

const MAX_LEN: usize = 32;
#[repr(transparent)]
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, rkyv::Portable)]
pub struct ArchivedLocalId([u8; MAX_LEN]);

impl Default for ArchivedLocalId {
    fn default() -> Self {
        Self([0; _])
    }
}

#[derive(Debug)]
pub enum LocalIdError {
    OverMaxLength,
    NonZeroEnd,
    InvalidUtf8(Utf8Error),
}

impl std::fmt::Display for LocalIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocalIdError::OverMaxLength => {
                write!(f, "The LocalId is over the {}-byte limit", MAX_LEN)
            }
            LocalIdError::NonZeroEnd => write!(
                f,
                "All excess bytes after the null terminator of a LocalId must also be 0x00"
            ),
            LocalIdError::InvalidUtf8(utf8_error) => write!(
                f,
                "Error occured whilst checking UTF-8 in LocalId: {utf8_error}"
            ),
        }
    }
}

impl core::error::Error for LocalIdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let LocalIdError::InvalidUtf8(utf8_error) = self {
            Some(utf8_error)
        } else {
            None
        }
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

unsafe impl<C> bytecheck::CheckBytes<C> for ArchivedLocalId
where
    C: rkyv::rancor::Fallible + ?Sized,
    C::Error: Source,
{
    unsafe fn check_bytes(value: *const Self, _: &mut C) -> Result<(), C::Error> {
        let c_str = value as *const u8;

        // SAFETY (for next three unsafe-s): We are always within the buffer allocated for this LocalId which is non-null and aligned.

        // Must find the null terminator in our string.
        let end = (0..MAX_LEN)
            .find(|i| unsafe { c_str.byte_add(*i).read() == 0x00 })
            .ok_or_else(|| C::Error::new(LocalIdError::OverMaxLength))?;

        // Check all bytes after null terminator until MAX_LEN are also 0x00.
        if !(end..MAX_LEN).all(|i| unsafe { c_str.byte_add(i).read() == 0x00 }) {
            return Err(C::Error::new(LocalIdError::NonZeroEnd));
        };

        let bytes = unsafe { core::slice::from_raw_parts(c_str, end) };

        // Validate that this is UTF-8
        let _ =
            str::from_utf8(bytes).map_err(|err| C::Error::new(LocalIdError::InvalidUtf8(err)))?;

        Ok(())
    }
}

unsafe impl NoUndef for ArchivedLocalId {}

impl ArchivedLocalId {
    pub const fn new(local_id: &str) -> Self {
        if local_id.len() >= MAX_LEN {
            panic!("Too long!");
        }

        let mut buf = [0u8; MAX_LEN];

        // SAFETY: local_id.len() < MAX_LEN
        unsafe {
            std::ptr::copy_nonoverlapping(local_id.as_ptr(), buf.as_mut_ptr(), local_id.len())
        };

        Self(buf)
    }

    pub fn as_str(&self) -> &str {
        CStr::from_bytes_until_nul(&self.0)
            .unwrap()
            .to_str()
            .unwrap()
    }
}
