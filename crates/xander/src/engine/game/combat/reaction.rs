use std::rc::{Rc, Weak};

use crate::engine::game::combat::{
    Combatant, action::Attack, arena::Position, utils::Availability,
};

use super::Timeslot;

#[derive(Debug)]
pub struct AttackOfOpportunity {
    pub to: Position,
    pub me: Weak<Combatant>,
    pub target: Weak<Combatant>,
}

impl AttackOfOpportunity {
    pub fn eligible_opportunity_attacks(self: &Rc<Self>) -> Vec<Availability<Weak<Attack>>> {
        let me: Rc<Combatant> = self.me.upgrade().unwrap();
        let target: Rc<Combatant> = self.target.upgrade().unwrap();

        let distance_before = target.distance_between(&me);
        let distance_after = me.distance_from(self.to);

        me.creature
            .stats
            .actions
            .attacks
            .attacks(
                &Timeslot::Reaction(Reaction::AttackOfOpportunity(self.clone())),
                &me,
                &target,
            )
            .into_iter()
            .map(|attack| {
                attack.and(|attack| {
                    let Some(attack): Option<Rc<Attack>> = attack.upgrade() else {
                        return false;
                    };

                    let as_reaction = attack.can_be_reaction();
                    let goes_out_of_range = {
                        let range = attack.range();

                        // Targets goes out of range of this attack.
                        range.within(distance_before) && !range.within(distance_after)
                    };

                    as_reaction && goes_out_of_range
                })
            })
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone)]
pub enum Reaction {
    AttackOfOpportunity(Rc<AttackOfOpportunity>),
}
