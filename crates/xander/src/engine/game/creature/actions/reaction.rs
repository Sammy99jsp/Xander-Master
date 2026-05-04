use std::rc::Rc;

use rkyv::with::Skip;
use xander_runtime::lived::LivedCell;

use crate::engine::game::{
    combat::{
        Combat,
        utils::{NextTurn, UntilNextTurn},
    },
    creature::Me,
};

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Reaction {
    pub me: Me,

    #[rkyv(with = Skip)]
    pub next_turn: LivedCell<UntilNextTurn>,
}

#[derive(Debug)]
pub struct AlreadyUsedReaction;

impl Reaction {
    pub const fn new(me: Me) -> Self {
        Self {
            me,
            next_turn: LivedCell::empty(),
        }
    }
    
    pub fn used(&self) -> bool {
        self.next_turn.is_inhabited()
    }

    pub fn mark_used(&self, combat: &Rc<Combat>) -> Result<(), AlreadyUsedReaction> {
        if self.used() {
            return Err(AlreadyUsedReaction);
        }

        self.next_turn.set(UntilNextTurn(NextTurn {
            start: combat.clock.rounds(),
            me: self.me.to_weak(),
            combat: Rc::downgrade(combat),
        }));

        Ok(())
    }
}
