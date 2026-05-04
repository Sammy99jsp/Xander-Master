use std::rc::Rc;

pub use schemars;
pub use serde_json;

use crate::engine::game::{
    combat::Combat,
    creature::{Creature, CreatureId},
};

pub mod creature;
pub mod stats;
pub mod utils;

impl Combat {
    pub fn load_raw_character(&self, me: creature::Creature) -> Rc<Creature> {
        let id = CreatureId(self.next_creature_id() as _);
        utils::WithId { value: me, id }.into()
    }

    pub fn next_creature_id(&self) -> usize {
        let id = self.creature_id.get();
        self.creature_id.update(|id| id + 1);
        id
    }
}
