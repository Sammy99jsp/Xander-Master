use std::{
    future::ready,
    ops::ControlFlow,
    rc::{Rc, Weak},
};

use dynx::Member;
use xander_runtime::{
    lived::{ArchivedProvisoBase, Proviso, ProvisoBase},
    register,
};

use crate::engine::game::{
    combat::Turn,
    creature::{
        Creature,
        marker::{ArchivedMarker, Marker},
    },
    measure::Feet,
};

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Dashing {
    pub turn: Weak<Turn>,
}

impl Dashing {
    pub async fn apply(self, me: &Creature) -> Rc<Self> {
        let rc = Rc::new(self);
        let dash = Rc::downgrade(&rc);
        me.stats.speed.enroll(DashEffect { dash });
        rc
    }
}

#[Member("ACTION::DASH", register(Archive, Deserialize))]
impl Marker for Dashing {}
impl ArchivedMarker for rkyv::Archived<Dashing> {}

register!(Dashing: dyn Marker, register(Lived(dependent(turn))));

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct DashEffect {
    pub dash: Weak<Dashing>,
}

register!(DashEffect: dyn ProvisoBase<Feet>, register(Identity("ACTION::DASHING"), Archive, Deserialize, Lived(dependent(dash))));

impl Proviso<Feet> for DashEffect {
    // Directly after base speed.
    const PRIORITY: usize = 1;
    fn provide(&self, t: &mut Feet) -> impl IntoFuture<Output = ControlFlow<()>> {
        // TODO: This could technically be incorrect since others may add their proviso before us.
        // Perhaps use a ValTree in future for Speed?
        t.0 *= 2;

        ready(ControlFlow::Continue(()))
    }
}
impl ArchivedProvisoBase<Feet> for rkyv::Archived<DashEffect> {}
