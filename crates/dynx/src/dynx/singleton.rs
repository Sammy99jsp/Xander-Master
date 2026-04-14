use std::ops::Deref;

use rkyv::{
    ptr_meta,
    rancor::Fallible,
    ser::Writer,
    string::{ArchivedString, StringResolver},
};

use crate::{
    IdentityBase, IntoNamespace,
    registry::{REGISTRY, Registered, StoredSingleton},
};

pub trait Singleton:
    Registered<StoredSingleton>
    + IntoNamespace
    + IdentityBase<Self::Namespace>
    + ptr_meta::Pointee<Metadata = ptr_meta::DynMetadata<Self>>
{
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Single<T>(pub(crate) &'static T)
where
    T: Singleton + ?Sized + 'static;

impl<T> std::fmt::Debug for Single<T>
where
    T: Singleton + ?Sized + 'static + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> std::fmt::Display for Single<T>
where
    T: Singleton + ?Sized + 'static + std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> Single<T>
where
    T: Singleton + ?Sized + 'static,
{
    pub fn new(value: &'static T) -> Self {
        Self(value)
    }
}

impl<T> Deref for Single<T>
where
    T: Singleton + ?Sized + 'static,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<T> Copy for Single<T> where T: Singleton + ?Sized + 'static {}
impl<T> Clone for Single<T>
where
    T: Singleton + ?Sized + 'static,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<Tr> rkyv::Archive for Single<Tr>
where
    Tr: Singleton + ?Sized,
{
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    fn resolve(&self, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
        ArchivedString::resolve_from_str(self.local_id(), resolver, out);
    }
}

impl<Tr, S> rkyv::Serialize<S> for Single<Tr>
where
    Tr: Singleton + ?Sized,
    S: Fallible + Writer + ?Sized,
    S::Error: rkyv::rancor::Source,
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, <S as Fallible>::Error> {
        ArchivedString::serialize_from_str(self.local_id(), serializer)
    }
}

impl<Tr, D> rkyv::Deserialize<Single<Tr>, D> for ArchivedString
where
    Tr: Singleton + ?Sized,
    D: Fallible + ?Sized,
    D::Error: rkyv::rancor::Source,
{
    fn deserialize(&self, _: &mut D) -> Result<Single<Tr>, <D as Fallible>::Error> {
        let local_id = self.as_str();
        let meta = REGISTRY
            .lookup::<Tr>(local_id)
            .expect("Should have found record");

        let erased = meta.stored_singleton.expect("Should have found singleton");
        unsafe { Ok(erased.cast::<Tr>()) }
    }
}
