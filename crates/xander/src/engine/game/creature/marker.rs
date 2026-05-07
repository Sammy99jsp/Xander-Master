use std::rc::Rc;

use dynx::{Identity, Namespace};
use xander_runtime::{Lived, lived::LivedList};

use crate::utils::IdentityExt;

#[Namespace("MARKER" @ NS, derive(Archive, Serialize, Deserialize, CheckBytes))]
pub trait Marker: Lived + std::fmt::Debug {}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Markers(LivedList<Rc<dyn Marker>>);

impl Markers {
    pub const fn new() -> Self {
        Self(LivedList::new())
    }

    pub fn push_mut<M>(&mut self, marker: Rc<M>)
    where
        M: Marker + 'static,
    {
        self.0.get_mut().push(marker);
    }

    pub fn push<M>(&self, marker: Rc<M>)
    where
        M: Marker + 'static,
    {
        self.0.push(marker);
    }

    pub fn contains<M>(&self) -> bool
    where
        M: Identity<Parent = dyn Marker>,
    {
        self.0.read().iter().any(|m| m.is::<M>())
    }

    pub fn iter(&self) -> impl Iterator<Item = Rc<dyn Marker>> {
        self.0
            .read()
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
    }
}

impl Default for Markers {
    fn default() -> Self {
        Self::new()
    }
}
