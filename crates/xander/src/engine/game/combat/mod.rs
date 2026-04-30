pub mod arena;
pub mod turn;
pub mod attack;

use std::{cell::Cell, rc::Rc};

use xander_runtime::dynx::cells::InnerValue;

use crate::engine::game::{combat::arena::Position, creature::Creature};

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Combatant {
    pub creature: Rc<Creature>,
    pub initiative_score: i32,

    #[rkyv(with = InnerValue<Position>)]
    pub position: Cell<Position>,
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Combat {
    initiative: Vec<Rc<Combatant>>,
}

impl Default for Combat {
    fn default() -> Self {
        Self::new()
    }
}

impl Combat {
    pub const fn new() -> Self {
        Self {
            initiative: Vec::new(),
        }
    }

    pub async fn enroll(&mut self, combatant: Combatant) {
        self.initiative.push(Rc::new(combatant));
        self.initiative.sort_by_cached_key(|c| c.initiative_score);
    }
}
