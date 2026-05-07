use std::rc::{Rc, Weak};

use dynx::Member;
use xander_runtime::register;

use crate::engine::game::{
    combat::Turn,
    creature::marker::{ArchivedMarker, Marker},
};

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Disengaging {
    pub turn: Weak<Turn>,
}

impl Disengaging {
    pub fn apply(self) -> Rc<Self> {
        Rc::new(self)
    }
}

#[Member("ACTION::DISENGAGING", register(Archive, Deserialize))]
impl Marker for Disengaging {}
impl ArchivedMarker for rkyv::Archived<Disengaging> {}

register!(Disengaging: dyn Marker, register(Lived(dependent(turn))));
