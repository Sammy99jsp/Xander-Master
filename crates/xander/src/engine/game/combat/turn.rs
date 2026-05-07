use std::{
    cell::Cell,
    ops::Add,
    rc::{Rc, Weak},
};

use thiserror::Error;
use xander_runtime::{dynx::cells::InnerValue, flow::Event, register};

use crate::engine::game::{
    combat::{
        Attack, Combat, Combatant, Timeslot,
        action::{
            Action, ActionType, NoActionLeft, dash::Dashing, disengage::Disengaging, dodge::Dodging,
        },
        arena::{self, Arena, Position, Square},
        attack::AttackReport,
        utils::{Availability, NextTurn},
    },
    creature::{CreatureSize, actions::AttackUseError},
    measure::{FEET_PER_SQUARE, Feet},
};

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Movement {
    #[rkyv(with = InnerValue<Feet>)]
    pub used: Cell<Feet>,
}

impl Movement {
    pub const fn new() -> Self {
        Self {
            used: Cell::new(Feet(0)),
        }
    }

    pub async fn used_up(&self, combatant: &Combatant) -> bool {
        self.used.get() >= combatant.creature.stats.speed.get().await
    }
}

impl Default for Movement {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Up = 0,
    TopRight = 1,
    Right = 2,
    BottomRight = 3,
    Bottom = 4,
    BottomLeft = 5,
    Left = 6,
    TopLeft = 7,
}

pub const DIRECTION_UNAVAILABLE: &str = "·";
pub const DIRECTION_ARROW: [&str; 8] = ["↑", "↗", "→", "↘", "↓", "↙", "←", "↖"];

pub const DIRECTIONS: [Direction; 8] = [
    Direction::Up,
    Direction::TopRight,
    Direction::Right,
    Direction::BottomRight,
    Direction::Bottom,
    Direction::BottomLeft,
    Direction::Left,
    Direction::TopLeft,
];

impl Add<Direction> for Position {
    type Output = Option<Position>;

    fn add(self, rhs: Direction) -> Self::Output {
        let [dx, dy] = arena::DIRECTIONS[rhs as u8 as usize];

        Some(Position {
            x: self.x.checked_add_signed(dx)?,
            y: self.y.checked_add_signed(dy)?,
        })
    }
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(serialize_bounds(__S: rkyv::ser::Writer + rkyv::ser::Sharing + rkyv::ser::Allocator, __S::Error: rkyv::rancor::Source))]
#[rkyv(deserialize_bounds(__D: rkyv::de::Pooling, __D::Error: rkyv::rancor::Source))]
#[rkyv(bytecheck(bounds(__C: rkyv::validation::ArchiveContext + rkyv::validation::SharedContext, __C::Error: rkyv::rancor::Source)))]
pub struct Turn {
    #[rkyv(omit_bounds)]
    pub combat: Weak<Combat>,
    pub me: Weak<Combatant>,
    pub movement: Movement,
    #[rkyv(with = InnerValue<Option<ActionType>>)]
    pub action: Cell<Option<ActionType>>,
}

register!(Turn, register(Identity("COMBAT::TURN"), Lived(@always)));

#[derive(Debug, Error)]
pub enum CannotMove {
    #[error("NO_MOVEMENT_LEFT")]
    NoMovementLeft,
    #[error("OUT_OF_BOUNDS")]
    OutOfBounds,
    #[error("SQUARE_OCCUPIED")]
    SquareOccupied,
}

impl Turn {
    pub async fn new(combat: &Rc<Combat>, combatant: &Rc<Combatant>) -> Rc<Self> {
        Rc::new(Self {
            combat: Rc::downgrade(combat),
            me: Rc::downgrade(combatant),
            movement: Movement::new(),
            action: Cell::new(None),
        })
    }

    pub async fn available_movement_directions(&self) -> Vec<Option<Direction>> {
        let me = self.me.upgrade().unwrap();
        if self.movement.used_up(&me).await {
            return vec![None; 8];
        }

        let arena: &Rc<Arena> = &self.combat.upgrade().unwrap().arena;

        let Some(around) = arena.around(me.position.get()) else {
            return vec![None; 8];
        };

        around
            .clockwise
            .into_iter()
            .enumerate()
            .map(|(i, sq)| sq.and_then(|sq| (!sq.is_occupied()).then_some(DIRECTIONS[i])))
            .collect()
    }

    pub async fn actions(self: &Rc<Self>) -> Vec<Availability<Action>> {
        Action::available_for_slot(&Timeslot::Turn(self.clone())).await
    }

    pub async fn movement_left(&self) -> Feet {
        let me: Rc<Combatant> = self.me.upgrade().unwrap();

        let used = self.movement.used.get();
        let speed = me.creature.stats.speed.get().await;

        speed - used
    }

    pub async fn move_in(&self, direction: Direction) -> Result<(), CannotMove> {
        let combat: Rc<Combat> = self.combat.upgrade().unwrap();
        let me = self.me.upgrade().unwrap();
        if self.movement.used_up(&me).await {
            return Err(CannotMove::NoMovementLeft);
        }

        match &me.creature.size {
            CreatureSize::Tiny | CreatureSize::Small | CreatureSize::Medium => (),
            _ => todo!("Bigger sizes that take up more than one square are not supported yet..."),
        }

        let Some(to) = me.position.get() + direction else {
            return Err(CannotMove::OutOfBounds);
        };

        let Some(to_sq): Option<&Square> = combat.arena.at(to) else {
            return Err(CannotMove::OutOfBounds);
        };

        if to_sq.is_occupied() {
            return Err(CannotMove::SquareOccupied);
        }

        let from = me.position.get();
        let from_sq: &Square = combat.arena.at(from).unwrap();

        // Fire the event...
        let me_weak = Rc::downgrade(&me);
        let result = events::PreMoveEvent {
            me: me_weak.clone(),
            from,
            to,
            cancelled: Default::default(),
        }
        .handle()
        .await
        .into_result();

        match result {
            Ok(()) => (),
            Err(events::MovedCancelledReason::Died) => {
                return Ok(());
            }
        }

        from_sq.remove_occupant(me_weak.clone());
        to_sq.add_occupant(me_weak.clone());

        {
            self.movement
                .used
                .update(|Feet(prev)| Feet(prev.saturating_add(FEET_PER_SQUARE)));

            me.position.set(to);
        }

        Ok(())
    }

    #[doc(hidden)]
    fn _try_use_action(
        &self,
        action: ActionType,
        extra: Option<Box<dyn FnOnce()>>,
    ) -> Result<ActionTransaction<'_>, NoActionLeft> {
        match (action, self.action.get()) {
            // Not taken an action yet, so we're okay...
            (_, None) => Ok(ActionTransaction {
                action,
                turn: self,
                extra,
            }),

            // We may have multi-attack...
            (ActionType::Attack, Some(ActionType::Attack)) => Ok(ActionTransaction {
                action,
                turn: self,
                extra,
            }),
            // But in the general case, we don't allow multiple actions per turn.
            (_, Some(_)) => Err(NoActionLeft),
        }
    }

    fn try_use_action(&self, action: ActionType) -> Result<ActionTransaction<'_>, NoActionLeft> {
        self._try_use_action(action, None)
    }

    pub async fn attack(
        self: &Rc<Self>,
        attack: Rc<Attack>,
        target: &Rc<Combatant>,
    ) -> Result<AttackReport, AttackUseError> {
        let trans = self.try_use_action(ActionType::Attack)?;

        let slot = Timeslot::Turn(self.clone());
        let me = self.me.upgrade().unwrap();
        if let Err(err) = attack.is_available(&slot, &me, target) {
            trans.cancel();
            return Err(err);
        }

        if let Err(err) = me
            .creature
            .stats
            .actions
            .attacks
            .left
            .use_attack(&slot)
            .await
        {
            trans.cancel();
            return Err(err);
        }

        let res = attack.attack(&slot, &me, target).await;
        match res {
            Err(err) => {
                trans.cancel();
                Err(err.into())
            }
            Ok(report) => Ok(report),
        }
    }

    pub async fn dash(self: &Rc<Self>) -> Result<(), NoActionLeft> {
        let _ = self.try_use_action(ActionType::Dash)?;
        let me: Rc<Combatant> = self.me.upgrade().unwrap();

        let dashing = Dashing {
            turn: Rc::downgrade(self),
        }
        .apply(&me.creature)
        .await;

        me.creature.stats.markers.push(dashing);

        Ok(())
    }

    pub async fn dodge(self: &Rc<Self>) -> Result<(), NoActionLeft> {
        let _ = self.try_use_action(ActionType::Dodge)?;
        let me: Rc<Combatant> = self.me.upgrade().unwrap();

        let dodging = Dodging {
            me: Rc::downgrade(&me.creature),
            next_turn: NextTurn::new(&me.creature).await,
        }
        .apply()
        .await;

        me.creature.stats.markers.push(dodging);

        Ok(())
    }

    pub async fn disengage(self: &Rc<Self>) -> Result<(), NoActionLeft> {
        let _ = self.try_use_action(ActionType::Disengage)?;
        let me: Rc<Combatant> = self.me.upgrade().unwrap();

        let disengaging = Disengaging {
            turn: Rc::downgrade(self),
        }
        .apply();

        me.creature.stats.markers.push(disengaging);

        Ok(())
    }
}

#[must_use = "You should bind this transaction to a variable to allow for cancelling it."]
pub struct ActionTransaction<'t> {
    action: ActionType,
    turn: &'t Turn,
    extra: Option<Box<dyn FnOnce()>>,
}

impl<'t> ActionTransaction<'t> {
    pub fn cancel(self) {
        std::mem::forget(self);
    }
}

impl<'t> Drop for ActionTransaction<'t> {
    fn drop(&mut self) {
        self.turn.action.set(Some(self.action));
        if let Some(other) = self.extra.take() {
            other()
        }
    }
}

pub mod events {
    use std::{
        future::ready,
        rc::{Rc, Weak},
    };

    use xander_runtime::{
        flow::event::{Event, EventBase, EventHandler, cancellable},
        register,
    };

    use crate::engine::{
        game::{
            Dispatcher, Game,
            combat::{
                Combatant, action::disengage::Disengaging, arena::Position,
                reaction::AttackOfOpportunity, utils::Availability,
            },
        },
        io::DynInterface,
    };

    #[derive(Debug)]
    pub enum MovedCancelledReason {
        Died,
    }

    // Mainly for Opportunity Attacks
    #[derive(Debug)]
    pub struct PreMoveEvent {
        pub me: Weak<Combatant>,
        pub from: Position,
        pub to: Position,
        pub(super) cancelled: Option<MovedCancelledReason>,
    }

    cancellable!(PreMoveEvent, cancelled);
    register!(PreMoveEvent: dyn EventBase<Game>, register(Identity("MOVE::PRE")));

    #[allow(clippy::unit_arg)]
    impl Event<Game> for PreMoveEvent {
        type Resolved = ();

        fn map_resolved(self) -> impl IntoFuture<Output = Self::Resolved> {
            ready(())
        }

        type Cancelled = MovedCancelledReason;

        fn map_cancelled(self) -> impl IntoFuture<Output = Self::Cancelled> {
            ready(self.cancelled.unwrap())
        }
    }

    #[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct OpportunityAttackHandler;

    register!(
        OpportunityAttackHandler,
        register(Identity("OPPORTUNITY_ATTACH_HANDLER"), Lived(@always))
    );

    impl EventHandler<Game> for OpportunityAttackHandler {
        type Event = PreMoveEvent;

        fn handle<'s, 'e: 's>(
            &'s self,
            event: &'e mut Self::Event,
        ) -> impl IntoFuture<Output = ()> + 's {
            async {
                #[deny(unused)]
                let me: Rc<Combatant> = event.me.upgrade().unwrap();

                if me.creature.stats.markers.contains::<Disengaging>() {
                    return;
                }

                let game = Dispatcher::local().await;
                for combatant in game.combat.initiative() {
                    // Skip ourselves.
                    if std::ptr::eq(Rc::as_ptr(&combatant), event.me.as_ptr()) {
                        continue;
                    }

                    // Skip Dead combatants
                    if combatant.creature.is_dead() {
                        continue;
                    }

                    if combatant.creature.stats.reaction.used() {
                        continue;
                    }

                    // Create the opportunity attack
                    let aoo = Rc::new(AttackOfOpportunity {
                        // The one performing the opportunity attack.
                        me: Rc::downgrade(&combatant),
                        // Targeting us.
                        target: Rc::downgrade(&me),
                        to: event.to,
                        combat: Rc::downgrade(&game.combat),
                    });

                    let eligible_attacks = aoo.actions().await;

                    let has_eligible_attacks =
                        eligible_attacks.iter().any(Availability::is_available);

                    if !has_eligible_attacks {
                        continue;
                    }

                    combatant
                        .opportunity_attack(aoo)
                        .await
                        .unwrap_or_else(|err| game.interface.error(err));

                    if me.creature.is_dead() {
                        event.cancelled = Some(MovedCancelledReason::Died);
                        return;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Weak};

    use crate::engine::game::combat::{
        Turn,
        action::{ActionType, NoActionLeft},
    };

    #[test]
    fn test_transaction() {
        let turn = Turn {
            combat: Weak::new(),
            me: Weak::new(),
            movement: super::Movement {
                used: Cell::default(),
            },
            action: Cell::default(),
        };

        let res = (|| -> Result<(), NoActionLeft> {
            let _transaction = turn.try_use_action(ActionType::Attack)?;
            Ok(())
        })();

        assert!(!res.is_err());
        assert!(turn.action.get().is_some());

        let res = (|| -> Result<(), NoActionLeft> {
            let _trans = turn.try_use_action(ActionType::Dash)?;
            Ok(())
        })();

        assert!(res.is_err());
    }

    #[test]
    fn test_transaction_cancelled() {
        let turn = Turn {
            combat: Weak::new(),
            me: Weak::new(),
            movement: super::Movement {
                used: Cell::default(),
            },
            action: Cell::default(),
        };

        let res = (|| -> Result<(), NoActionLeft> {
            let trans = turn.try_use_action(ActionType::Attack)?;
            trans.cancel();
            Ok(())
        })();

        assert!(!res.is_err());
        assert!(turn.action.get().is_none());

        let res = (|| -> Result<(), NoActionLeft> {
            let _trans = turn.try_use_action(ActionType::Dash)?;
            Ok(())
        })();

        assert!(res.is_ok());
        assert!(turn.action.get().is_some());
    }
}
