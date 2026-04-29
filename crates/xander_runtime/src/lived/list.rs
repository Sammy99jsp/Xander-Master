use std::cell::{BorrowError, Ref, RefCell, RefMut};

use rkyv::{
    Archive, Deserialize, Serialize,
    rancor::{Fallible, Source},
    ser::{Allocator, Writer},
    vec::ArchivedVec,
};

use crate::lived::Lived;

/// A list of [Lived] objects, which lazily drops objects,
/// when queried, that are no longer [Lived::is_alive].
#[derive(Debug)]
pub struct LivedList<L> {
    contents: RefCell<Vec<L>>,
}

impl<L> Default for LivedList<L> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L> LivedList<L> {
    pub const fn new() -> Self {
        Self {
            contents: RefCell::new(Vec::new()),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            contents: RefCell::new(Vec::with_capacity(capacity)),
        }
    }
}

impl<L> LivedList<L>
where
    L: Lived,
{
    fn cleanup(&self) {
        cleanup(self.contents.borrow_mut().as_mut());
    }

    pub fn read(&self) -> Ref<'_, Vec<L>> {
        self.cleanup();
        self.contents.borrow()
    }

    pub fn try_read(&self) -> Result<Ref<'_, Vec<L>>, BorrowError> {
        self.contents.try_borrow()
    }

    pub fn write(&self) -> RefMut<'_, Vec<L>> {
        self.cleanup();
        self.contents.borrow_mut()
    }

    pub fn get_mut(&mut self) -> &mut Vec<L> {
        let contents = self.contents.get_mut();
        cleanup(contents);
        contents
    }

    pub fn push(&self, value: L) {
        self.write().push(value);
    }
}

fn cleanup<L>(contents: &mut Vec<L>)
where
    L: Lived,
{
    let to_delete = {
        contents
            .iter()
            .enumerate()
            .filter_map(|(i, a)| (!a.is_alive()).then_some(i))
            .collect::<Vec<_>>()
    };

    if !to_delete.is_empty() {
        // Remove all !is_alive(...) items.
        for (offset, i) in to_delete.into_iter().enumerate() {
            contents.remove(i - offset);
        }
    }
}

impl<L> Archive for LivedList<L>
where
    L: Lived + Archive,
{
    type Archived = rkyv::Archived<Vec<L>>;
    type Resolver = rkyv::Resolver<Vec<L>>;

    fn resolve(&self, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
        ArchivedVec::resolve_from_len(self.read().len(), resolver, out);
    }
}

impl<L, S> Serialize<S> for LivedList<L>
where
    S: Fallible + Writer + Allocator + ?Sized,
    L: Lived + Serialize<S>,
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::serialize_from_slice(self.read().as_slice(), serializer)
    }
}

impl<L, D> rkyv::Deserialize<LivedList<L>, D> for ArchivedVec<L::Archived>
where
    D: Fallible + ?Sized,
    D::Error: Source,
    L: Lived + Archive,
    L::Archived: Deserialize<L, D>,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<LivedList<L>, <D as Fallible>::Error> {
        let inner =
            <ArchivedVec<L::Archived> as Deserialize<Vec<L>, D>>::deserialize(self, deserializer)?;

        Ok(LivedList {
            contents: RefCell::new(inner),
        })
    }
}
