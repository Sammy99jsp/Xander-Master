pub mod attack;
pub mod dash;
pub mod disengage;
pub mod dodge;

use std::rc::{Rc, Weak};

use thiserror::Error;

use crate::engine::game::{
    Dispatcher, Game,
    combat::{Combatant, Reaction, Timeslot, utils::Availability},
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

pub const SUPPORTED_NON_ATTACK_ACTIONS: [Action; 3] =
    [Action::Dash, Action::Disengage, Action::Dodge];

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

fn attacks(
    game: &Game,
    me: &Rc<Combatant>,
    slot: &Timeslot,
) -> impl Iterator<Item = Availability<Action>> {
    let me_weak = Rc::downgrade(me);
    game.combat
        .initiative()
        .into_iter()
        .filter(move |target| !Rc::ptr_eq(me, target))
        .flat_map(|target| {
            me.creature
                .stats
                .actions
                .attacks
                .attacks(slot, me, &target)
                .into_iter()
                .map(move |attack| (Rc::downgrade(&target), target.creature.is_dead(), attack))
        })
        .map(move |(target, is_dead, attack)| {
            attack
                .map(|attack| {
                    Action::Attack(Attacking {
                        me: me_weak.clone(),
                        target,
                        attack,
                    })
                })
                .and(|_| !is_dead)
        })
}

impl Action {
    pub async fn actions_for(me: &Rc<Combatant>) -> impl Iterator<Item = Action> {
        let game = Dispatcher::local().await;

        SUPPORTED_NON_ATTACK_ACTIONS
            .into_iter()
            .chain(attacks(game, me, &Timeslot::Any).map(|a| a.value()))
    }

    pub async fn available_for_slot(slot: &Timeslot) -> Vec<Availability<Action>> {
        let me: Rc<Combatant> = slot.me().upgrade().unwrap();
        let game = Dispatcher::local().await;

        let non_attacks = SUPPORTED_NON_ATTACK_ACTIONS.map(|action| match slot {
            Timeslot::Any => Availability::available(action),
            Timeslot::Turn(turn) if turn.action.get().is_none() => Availability::available(action),
            Timeslot::Reaction(_) | Timeslot::Turn(_) => Availability::unavailable(action),
        });

        let can_attack = match slot {
            slot @ Timeslot::Reaction(Reaction::AttackOfOpportunity(_)) => {
                me.creature
                    .stats
                    .actions
                    .attacks
                    .left
                    .can_attack(slot)
                    .await
            }
            slot @ Timeslot::Turn(turn) if let Some(ActionType::Attack) = turn.action.get() => {
                me.creature
                    .stats
                    .actions
                    .attacks
                    .left
                    .can_attack(slot)
                    .await
            }
            Timeslot::Turn(turn) if turn.action.get().is_none() => true,
            Timeslot::Turn(_) => false,
            Timeslot::Any => true,
        };

        let attacks = attacks(game, &me, slot).map(|availability| availability.and(|_| can_attack));

        non_attacks.into_iter().chain(attacks).collect()
    }
}

#[derive(Debug, Clone)]
pub struct Attacking {
    pub me: Weak<Combatant>,
    pub target: Weak<Combatant>,
    pub attack: Weak<Attack>,
}
