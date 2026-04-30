use xander_runtime::dynx::rkyv::{
    Archive, Archived, Deserialize, Portable, Resolver, Serialize,
    rancor::{Fallible, Source},
    ser::{Allocator, Writer},
    with::{ArchiveWith, DeserializeWith, SerializeWith},
};

use crate::*;

pub struct Unlabeled;

#[repr(transparent)]
#[derive(rkyv::bytecheck::CheckBytes)]
#[bytecheck(crate = xander_runtime::dynx::bytecheck)]
pub struct ArchivedWrapper<T: Archive>(Archived<Box<T>>);
unsafe impl<T: Archive> Portable for ArchivedWrapper<T> {}

impl<T: Archive> ArchiveWith<Labeled<T>> for Unlabeled {
    type Archived = ArchivedWrapper<T>;
    type Resolver = Resolver<Box<T>>;

    fn resolve_with(
        Labeled(expr, _): &Labeled<T>,
        resolver: Self::Resolver,
        out: rkyv::Place<Self::Archived>,
    ) {
        rkyv::munge::munge!(let ArchivedWrapper(inner) = out);
        expr.resolve(resolver, inner)
    }
}

impl<S, T> SerializeWith<Labeled<T>, S> for Unlabeled
where
    S: Fallible + Writer + Allocator + ?Sized,
    T: Serialize<S>,
{
    fn serialize_with(
        Labeled(expr, _): &Labeled<T>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        expr.serialize(serializer)
    }
}

impl<D, T> DeserializeWith<ArchivedWrapper<T>, Labeled<T>, D> for Unlabeled
where
    D: Fallible + ?Sized,
    D::Error: Source,
    T: Archive,
    T::Archived: Deserialize<T, D>,
{
    fn deserialize_with(
        ArchivedWrapper(field): &ArchivedWrapper<T>,
        deserializer: &mut D,
    ) -> Result<Labeled<T>, D::Error> {
        let expr = field.deserialize(deserializer)?;
        Ok(Labeled(expr, Label(None)))
    }
}
