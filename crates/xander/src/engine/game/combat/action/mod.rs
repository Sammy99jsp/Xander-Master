pub mod attack;
pub mod dash;
pub mod disengage;
pub mod dodge;

use std::rc::{Rc, Weak};

use thiserror::Error;

use crate::engine::game::{
    Dispatcher,
    combat::{Combatant, Turn, utils::Availability},
};

pub use attack::Attack;

#[derive(Debug, Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum ActionType {
    Attack = 1,
    Dash,
    Disengage,
    Dodge,
    Help,
    Hide,
    Influence,
    Magic,
    Ready,
    Search,
    Study,
    Utilize,
}

#[derive(Debug, Error)]
#[error("NO_ACTION_LEFT")]
pub struct NoActionLeft;

#[derive(Debug, Clone)]
pub enum Action {
    Dash,
    Disengage,
    Dodge,
    Attack(Attacking),
    // Magic,
}

impl PartialEq<ActionType> for Action {
    fn eq(&self, other: &ActionType) -> bool {
        matches!(
            (self, other),
            (Action::Dash, ActionType::Dash)
                | (Action::Disengage, ActionType::Disengage)
                | (Action::Dodge, ActionType::Dodge)
                | (Action::Attack(_), ActionType::Attack)
        )
    }
}

impl Action {
    pub async fn available_for_turn(turn: &Rc<Turn>) -> Vec<Availability<Action>> {
        let slot = &super::Timeslot::Turn(turn.clone());
        let game = Dispatcher::local().await;
        let me_weak = turn.me.clone();
        let me: Rc<Combatant> = turn.me.upgrade().unwrap();

        let non_attacks =
            [Action::Dash, Action::Disengage, Action::Dodge].map(|action| {
                match turn.action.get().is_none() {
                    true => Availability::available(action),
                    false => Availability::unavailable(action),
                }
            });

        let can_attack = match turn.action.get() {
            None => true,
            Some(ActionType::Attack) => {
                me.creature
                    .stats
                    .actions
                    .attacks
                    .left
                    .can_attack(slot)
                    .await
            }
            _ => false,
        };

        let attacks = game
            .combat
            .initiative()
            .into_iter()
            .filter(|target| !Rc::ptr_eq(&me, target) && !target.creature.is_dead())
            .flat_map(|target| {
                me.creature
                    .stats
                    .actions
                    .attacks
                    .attacks(slot, &me, &target)
                    .into_iter()
                    .map(move |attack| (Rc::downgrade(&target), attack))
            })
            .map(|(target, attack)| {
                attack.map(|attack| {
                    Action::Attack(Attacking {
                        me: me_weak.clone(),
                        target,
                        attack,
                    })
                })
            })
            .map(|availability| availability.and(|_| can_attack));

        non_attacks.into_iter().chain(attacks).collect()
    }
}

#[derive(Debug, Clone)]
pub struct Attacking {
    pub me: Weak<Combatant>,
    pub target: Weak<Combatant>,
    pub attack: Weak<Attack>,
}
