use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
};

use rkyv::{
    Archive, Deserialize, Serialize,
    rancor::Fallible,
    with::{ArchiveWith, DeserializeWith, SerializeWith},
};

pub struct InnerValue<T>(PhantomData<T>);

impl<T> ArchiveWith<Cell<T>> for InnerValue<T>
where
    T: Copy + Archive,
{
    type Archived = T::Archived;
    type Resolver = T::Resolver;

    fn resolve_with(field: &Cell<T>, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
        let value = field.get();
        value.resolve(resolver, out);
    }
}

impl<T, S> SerializeWith<Cell<T>, S> for InnerValue<T>
where
    T: Copy + Serialize<S>,
    S: Fallible + ?Sized,
{
    fn serialize_with(field: &Cell<T>, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        field.get().serialize(serializer)
    }
}

impl<T, D> DeserializeWith<T::Archived, Cell<T>, D> for InnerValue<T>
where
    T: Copy + Archive,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(field: &T::Archived, deserializer: &mut D) -> Result<Cell<T>, D::Error> {
        Ok(Cell::new(field.deserialize(deserializer)?))
    }
}

impl<T> ArchiveWith<RefCell<T>> for InnerValue<T>
where
    T: Archive,
{
    type Archived = T::Archived;
    type Resolver = T::Resolver;

    fn resolve_with(
        field: &RefCell<T>,
        resolver: Self::Resolver,
        out: rkyv::Place<Self::Archived>,
    ) {
        let value = field.borrow();
        value.resolve(resolver, out);
    }
}

impl<T, S> SerializeWith<RefCell<T>, S> for InnerValue<T>
where
    T: Serialize<S>,
    S: Fallible + ?Sized,
{
    fn serialize_with(field: &RefCell<T>, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        field.borrow().serialize(serializer)
    }
}

impl<T, D> DeserializeWith<T::Archived, RefCell<T>, D> for InnerValue<T>
where
    T: Archive,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(field: &T::Archived, deserializer: &mut D) -> Result<RefCell<T>, D::Error> {
        Ok(RefCell::new(field.deserialize(deserializer)?))
    }
}
