use std::rc::Rc;

use crate::engine::game::creature::Creature;

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Combatant {
    pub creature: Rc<Creature>,
    pub initiative_score: i32,
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Combat {
    initiative: Vec<Combatant>,
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

    pub async fn enroll(&self, combatant: Combatant) -> ! {
        todo!("Sort out the whole decision rules we're going with here.")
    }
}
