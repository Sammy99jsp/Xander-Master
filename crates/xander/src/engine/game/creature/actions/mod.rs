pub mod reaction;

use std::{
    cell::Cell,
    rc::{Rc, Weak},
};

use thiserror::Error;
use xander_runtime::{
    dynx::cells::InnerValue,
    lived::{LivedList, Provided},
};

use crate::engine::{
    game::{
        combat::{
            Combatant, Timeslot,
            action::{Attack, NoActionLeft},
            turn::Turn,
            utils::Availability,
        },
        creature::Me,
    },
    io::roller::DiceRollerError,
};

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Actions {
    pub attacks: Attacks,
}

impl Actions {
    pub fn new(me: Me) -> Self {
        Self {
            attacks: Attacks::new(me),
        }
    }
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Attacks {
    me: Me,
    max_attacks: Provided<u32>,
    pub left: AttacksLeft,
    pub attacks: LivedList<Rc<Attack>>,
}

const DEFAULT_NUM_ATTACKS: u32 = 1;

impl Attacks {
    pub fn new(me: Me) -> Self {
        Self {
            me: me.clone(),
            left: AttacksLeft::new(me),
            max_attacks: {
                let mut provided = Provided::new();
                provided.enroll_mut(proviso::SetMaxAttacks(DEFAULT_NUM_ATTACKS));
                provided
            },
            attacks: LivedList::new(),
        }
    }

    pub fn attacks(
        &self,
        slot: &Timeslot,
        me: &Rc<Combatant>,
        target: &Rc<Combatant>,
    ) -> Vec<Availability<Weak<Attack>>> {
        self.attacks
            .read()
            .iter()
            .map(|a| match a.is_available(slot, me, target).is_ok() {
                true => Availability::available(Rc::downgrade(a)),
                false => Availability::unavailable(Rc::downgrade(a)),
            })
            .collect()
    }
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct AttacksLeft {
    me: Me,
    #[rkyv(with = InnerValue<u32>)]
    count: Cell<u32>,
}

#[derive(Debug, Error)]
pub enum AttackUseError {
    #[error("NO_ACTION_LEFT")]
    NoActionLeft(#[from] NoActionLeft),
    #[error("OUT_OF_TURN")]
    OutOfTurn,
    #[error("OUT_OF_ATTACKS")]
    OutOfAttacks,
    #[error("OUT_OF_RANGE")]
    OutOfRange,
    #[error("OUT_OF_BOUNDS")]
    OutOfBounds,
    #[error("NO_TARGET")]
    NoTarget,
    #[error("TARGETING_SELF")]
    TargetingSelf,
    #[error("DICE_ROLLING")]
    DiceRolling(#[from] DiceRollerError),
}

impl AttacksLeft {
    pub fn can_attack(&self) -> bool {
        self.count.get() > 0
    }

    pub fn use_attack(&self) -> Result<(), AttackUseError> {
        // Check if we have already used up all of our attacks.
        if !self.can_attack() {
            return Err(AttackUseError::OutOfAttacks);
        }

        // Update the count:
        self.count.update(|attacks| attacks.saturating_sub(1));

        Ok(())
    }

    pub async fn reset(&self, _: &Rc<Turn>) {
        self.count
            .set(self.me.stats.actions.attacks.max_attacks.get().await)
    }

    pub fn new(me: Me) -> Self {
        Self {
            me,
            count: Cell::new(DEFAULT_NUM_ATTACKS),
        }
    }
}

pub mod proviso {
    use std::future::ready;

    use xander_runtime::{lived::provided::prelude::*, register};

    #[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct SetMaxAttacks(pub u32);

    register!(SetMaxAttacks: dyn ProvisoBase<u32>, register(Identity("SET_MAX_ATTACKS"), Archive, Deserialize, Lived(always)));

    impl ArchivedProvisoBase<u32> for rkyv::Archived<SetMaxAttacks> {}

    impl Proviso<u32> for SetMaxAttacks {
        fn provide(&self, t: &mut u32) -> impl IntoFuture<Output = ControlFlow<()>> {
            *t = self.0;
            ready(ControlFlow::Continue(()))
        }
    }
}
