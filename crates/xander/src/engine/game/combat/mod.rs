pub mod action;
pub mod arena;
pub mod reaction;
pub mod turn;
pub mod utils;
pub mod win;

use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use smol::future::FutureExt;
use thiserror::Error;
use xander_runtime::{
    dynx::cells::InnerValue,
    flow::{dispatcher::DispatchState, io::Actor},
};

use crate::engine::game::{
    Dispatcher, Game,
    combat::{
        arena::{Arena, Position},
        reaction::AttackOfOpportunity,
        turn::events::OpportunityAttackHandler,
        win::WinCondition,
    },
    creature::Creature,
    measure::{
        Feet,
        time::{Rounds, Turns},
    },
    stats::d20_test::{Check, Dc},
};

pub use action::attack::{self, Attack};

pub use reaction::Reaction;
pub use turn::Turn;

use super::stats::Ability;

#[derive(Debug, Clone)]
pub enum Timeslot {
    Turn(Rc<Turn>),
    Reaction(Reaction),
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Combat {
    #[rkyv(with = InnerValue<bool>)]
    pub started: Cell<bool>,

    pub clock: CombatClock,

    pub arena: Rc<Arena>,

    #[rkyv(with = InnerValue<Option<Rc<Turn>>>)]
    current_turn: RefCell<Option<Rc<Turn>>>,
    #[rkyv(with = InnerValue<Vec<Rc<Combatant>>>)]
    initiative: RefCell<Vec<Rc<Combatant>>>,

    #[rkyv(with = InnerValue<usize>)]
    pub creature_id: Cell<usize>,

    pub win_condition: WinCondition,
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct CombatClock {
    #[rkyv(with = InnerValue<Rounds>)]
    rounds: Cell<Rounds>,

    #[rkyv(with = InnerValue<Turns>)]
    turns: Cell<Turns>,

    #[rkyv(with = InnerValue<usize>)]
    turn_order: Cell<usize>,
}

impl CombatClock {
    pub const fn new() -> Self {
        Self {
            rounds: Cell::new(Rounds(0)),
            turns: Cell::new(Turns(0)),
            turn_order: Cell::new(0),
        }
    }

    pub const fn rounds(&self) -> Rounds {
        self.rounds.get()
    }

    pub const fn turns(&self) -> Turns {
        self.turns.get()
    }

    pub const fn current_turn_order(&self) -> usize {
        self.turn_order.get()
    }

    fn tick(&self, round_len: usize) {
        let next_round = self.turn_order.get() >= (round_len - 1);

        if next_round {
            self.rounds.update(|Rounds(r)| Rounds(r + 1));
            self.turn_order.set(0);
        } else {
            self.turn_order.update(|i| i + 1);
        }

        self.turns.update(|Turns(t)| Turns(t + 1));
    }
}

impl Default for CombatClock {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Combatant {
    pub creature: Rc<Creature>,
    pub actor: Actor,
    #[rkyv(with = InnerValue<Position>)]
    pub position: Cell<Position>,

    #[rkyv(with = InnerValue<i32>)]
    pub initiative_score: Cell<i32>,
}

impl Combatant {
    pub fn is(&self, creature: &Weak<Creature>) -> bool {
        std::ptr::eq(Rc::as_ptr(&self.creature), creature.as_ptr())
    }

    pub fn distance_from(&self, location: Position) -> Feet {
        Feet::from(Arena::distance(self.position.get(), location))
    }

    pub fn distance_between(&self, other: &Combatant) -> Feet {
        Feet::from(Arena::distance(self.position.get(), other.position.get()))
    }

    async fn opportunity_attack(
        self: &Rc<Self>,
        aoo: Rc<AttackOfOpportunity>,
    ) -> Result<(), Box<dyn Any>> {
        let game = Dispatcher::local().await;
        let agent = self.actor.state(&game.interface);
        agent.opportunity_attack(aoo).await?;

        Ok(())
    }

    async fn roll_for_initiative(self: &Rc<Self>) {
        // TODO: Do the work necessary to label this check with an Initiative marker.
        let ev = self
            .check(Check {
                ability: Ability::Dexterity,
                prof: None,
                dc: Some(Dc(d20::DExpr::from(100))),
            })
            .await
            .into_result()
            .unwrap();

        self.initiative_score.set(ev.roll_result.total());
    }
}

#[derive(Debug, Error)]
pub enum EnrollCombatantError {
    #[error("OUT_OF_BOUNDS")]
    OutOfBounds,
}

impl Combat {
    pub fn new(arena: Arena) -> Self {
        Self {
            started: Cell::new(false),
            arena: Rc::new(arena),
            initiative: RefCell::new(Vec::new()),
            clock: CombatClock::new(),
            current_turn: RefCell::new(None),
            creature_id: Cell::new(0),
            win_condition: WinCondition::FreeForAll,
        }
    }

    #[must_use = "You must check the result of this operation."]
    pub async fn enroll(
        &self,
        creature: Rc<Creature>,
        actor: Actor,
        position: Position,
    ) -> Result<Weak<Combatant>, EnrollCombatantError> {
        let combatant = Rc::new(Combatant {
            creature,
            actor,
            position: Cell::new(position),
            initiative_score: Cell::new(0),
        });

        combatant.roll_for_initiative().await;

        let Some(sq) = self.arena.at(combatant.position.get()) else {
            return Err(EnrollCombatantError::OutOfBounds);
        };

        let mut initiative = self.initiative.borrow_mut();

        let ret = Rc::downgrade(&combatant);

        initiative.push(combatant);
        initiative.sort_by_cached_key(|c| -c.initiative_score.get());

        sq.add_occupant(ret.clone());

        Ok(ret)
    }

    pub fn turn_order_of(&self, creature: &Weak<Creature>) -> usize {
        self.initiative
            .borrow()
            .iter()
            .position(|c| std::ptr::eq(Rc::as_ptr(&c.creature), creature.as_ptr()))
            .unwrap()
    }

    pub fn current_turn(&self) -> Rc<Turn> {
        self.current_turn.borrow().clone().unwrap()
    }

    pub fn initiative(&self) -> Vec<Rc<Combatant>> {
        self.initiative.borrow().clone()
    }

    pub fn len_members(&self) -> usize {
        self.initiative.borrow().len()
    }

    pub async fn member_status(&self) {
        let members = self.initiative.borrow().clone();

        for member in members {
            println!(
                "<{} {}/{}>",
                member.creature.name,
                member.creature.stats.health.current(),
                member.creature.stats.health.max_hp.get().await
            );
        }
    }

    pub async fn termination_condition(&self, game: &Game) -> Result<bool, Box<dyn Any>> {
        if self.len_members() <= 1 {
            return Ok(true);
        }

        let winners = self.win_condition.has_happened(self);

        let Some(winners) = winners else {
            return Ok(false);
        };

        let mut losers = self.initiative();
        losers.retain(|loser| !winners.iter().any(|winner| Rc::ptr_eq(winner, loser)));

        for winner in winners {
            let agent = winner.actor.state(&game.interface);
            agent.game_end(win::GameEndReport { won: true }).await?;
        }

        for loser in losers {
            let agent = loser.actor.state(&game.interface);
            agent.game_end(win::GameEndReport { won: false }).await?;
        }

        Ok(true)
    }

    pub fn start<'s, 'g>(
        &'s self,
        game: &'g Game,
    ) -> impl IntoFuture<Output = Result<(), Box<dyn Any>>> + 'g
    where
        's: 'g,
    {
        if self.started.get() {
            panic!("Do not start the game twice!");
        }

        game.listen(OpportunityAttackHandler);
        game.dispatcher
            .dispatch(async {
                self.started.set(true);

                let max_iter: Turns = Turns(100_000);
                while self.clock.turns() < max_iter {
                    game.update().await?;
                    if self.termination_condition(game).await? {
                        break;
                    }

                    self.clock.tick(self.len_members());
                    // println!("{}", self.arena.display_debug());
                    // println!("Turn: {:?}", self.clock.turns());

                    let i = self.clock.current_turn_order();
                    let combatant = self.initiative.borrow().get(i).unwrap().clone();

                    if !combatant.creature.can_take_turns() {
                        continue;
                    }

                    let turn = Turn::new(self, &combatant).await;
                    let weak = Rc::downgrade(&turn);
                    self.current_turn.borrow_mut().replace(turn);

                    combatant.actor.state(&game.interface).turn(weak).await?;
                }

                Ok(())
            })
            .boxed_local()
    }
}
