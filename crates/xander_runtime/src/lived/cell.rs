use std::cell::{Ref, RefCell, RefMut};

use rkyv::{Archive, Deserialize, Serialize, option::ArchivedOption, rancor::Fallible};

use super::Lived;

/// A container that takes a single possible [Lived] value.
///
/// If at any point, the value is not [Lived::is_alive], it is dropped,
/// leaving the cell empty.
#[derive(Debug)]
pub struct LivedCell<L>(RefCell<Option<L>>)
where
    L: Lived;

impl<L> LivedCell<L>
where
    L: Lived,
{
    pub const fn empty() -> Self {
        Self(RefCell::new(None))
    }

    pub const fn new(val: L) -> Self {
        Self(RefCell::new(Some(val)))
    }

    fn cleanup(&self) {
        cleanup(&mut *self.0.borrow_mut())
    }

    pub fn get(&self) -> Ref<'_, Option<L>> {
        self.cleanup();
        self.0.borrow()
    }

    pub fn get_mut(&mut self) -> &mut Option<L> {
        let inner = self.0.get_mut();
        cleanup(inner);
        inner
    }

    pub fn set(&self, val: L) {
        let _ = self.write().replace(val);
    }

    pub fn write(&self) -> RefMut<'_, Option<L>> {
        self.cleanup();
        self.0.borrow_mut()
    }

    pub fn is_inhabited(&self) -> bool {
        self.get().is_some()
    }
}

impl<L> Default for LivedCell<L>
where
    L: Lived,
{
    fn default() -> Self {
        Self::empty()
    }
}

fn cleanup<L>(inner: &mut Option<L>)
where
    L: Lived,
{
    // If the value is not alive, drop it.
    if inner.as_ref().is_some_and(|a| !a.is_alive()) {
        let _ = inner.take();
    }
}

impl<L> Archive for LivedCell<L>
where
    L: Lived + Archive,
{
    type Archived = rkyv::Archived<Option<L>>;
    type Resolver = rkyv::Resolver<Option<L>>;

    fn resolve(&self, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
        self.0.borrow().resolve(resolver, out);
    }
}

impl<L, S> Serialize<S> for LivedCell<L>
where
    L: Lived + Serialize<S>,
    S: Fallible + ?Sized,
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        self.0.borrow().serialize(serializer)
    }
}

impl<L, D> rkyv::Deserialize<LivedCell<L>, D> for ArchivedOption<L::Archived>
where
    D: Fallible + ?Sized,
    L: Lived + Archive,
    L::Archived: Deserialize<L, D>,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<LivedCell<L>, <D as Fallible>::Error> {
        let inner: Option<L> = self.deserialize(deserializer)?;
        Ok(LivedCell(RefCell::new(inner)))
    }
}
