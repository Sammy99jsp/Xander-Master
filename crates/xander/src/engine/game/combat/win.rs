use std::rc::Rc;

use crate::engine::game::combat::{Combat, Combatant};

#[derive(Debug, Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum WinCondition {
    FreeForAll,
}

impl WinCondition {
    pub fn has_happened(&self, combat: &Combat) -> Option<Vec<Rc<Combatant>>> {
        match self {
            WinCondition::FreeForAll => {
                let combatants = combat.initiative.borrow();

                let mut iter = combatants.iter().filter(|c| !c.creature.is_dead());

                let Some(potential_winner) = iter.next() else {
                    // Everybody's dead at the same time. I guess everyone loses.
                    return Some(vec![]);
                };

                if iter.count() > 0 {
                    // Too many alive still.
                    return None;
                }

                // Only one combatant still alive, so they win!
                Some(vec![potential_winner.clone()])
            }
        }
    }
}

#[derive(Debug)]
pub struct GameEndReport {
    pub won: bool,
}