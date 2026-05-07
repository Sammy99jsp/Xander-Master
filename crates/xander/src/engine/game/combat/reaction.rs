use std::rc::{Rc, Weak};

use crate::engine::game::{
    combat::{
        Combat, Combatant,
        action::{Action, Attack, Attacking},
        arena::Position,
        attack::AttackReport,
        utils::Availability,
    },
    creature::actions::AttackUseError,
};

use super::Timeslot;

#[derive(Debug)]
pub struct AttackOfOpportunity {
    pub to: Position,
    pub combat: Weak<Combat>,
    pub me: Weak<Combatant>,
    pub target: Weak<Combatant>,
}

impl AttackOfOpportunity {
    pub async fn actions(self: &Rc<Self>) -> Vec<Availability<Action>> {
        let me: Rc<Combatant> = self.me.upgrade().unwrap();

        let distance_before = self.target.upgrade().unwrap().distance_to(&me);
        let distance_after = me.distance_from(self.to);

        let slot = Timeslot::Reaction(Reaction::AttackOfOpportunity(self.clone()));
        Action::available_for_slot(&slot)
            .await
            .into_iter()
            .map(|action| {
                action.and(|action| {
                    match action {
                        Action::Dash | Action::Disengage | Action::Dodge => false,
                        Action::Attack(Attacking { target, attack, .. }) => {
                            if !target.ptr_eq(&self.target) {
                                return false;
                            }

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
                        }
                    }
                })
            })
            .collect::<Vec<_>>()
    }

    pub async fn attack(
        self: &Rc<Self>,
        attack: &Rc<Attack>,
    ) -> Result<AttackReport, AttackUseError> {
        let slot = Timeslot::Reaction(Reaction::AttackOfOpportunity(self.clone()));
        let me: Rc<Combatant> = self.me.upgrade().unwrap();
        let target = self.target.upgrade().unwrap();
        attack.is_available(&slot, &me, &target)?;
        Ok(attack.attack(&slot, &me, &target).await?)
    }
}

#[derive(Debug, Clone)]
pub enum Reaction {
    AttackOfOpportunity(Rc<AttackOfOpportunity>),
}

impl Reaction {
    pub fn me(&self) -> Rc<Combatant> {
        match self {
            Reaction::AttackOfOpportunity(attack_of_opportunity) => {
                attack_of_opportunity.me.upgrade().unwrap()
            }
        }
    }

    pub fn combat(&self) -> Rc<Combat> {
        match self {
            Reaction::AttackOfOpportunity(attack_of_opportunity) => {
                attack_of_opportunity.combat.upgrade().unwrap()
            }
        }
    }
}
