use std::{array, cell::Cell, ops::Add, rc::Weak};

use xander_runtime::dynx::cells::InnerValue;

use crate::engine::game::{
    combat::{
        Combatant,
        arena::{self, Arena, Position},
    },
    creature::CreatureSize,
    measure::{FEET_PER_SQUARE, Feet},
};

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Movement {
    #[rkyv(with = InnerValue<Feet>)]
    left: Cell<Feet>,
}

impl Movement {
    pub fn any_left(&self) -> bool {
        self.left.get() >= Feet(FEET_PER_SQUARE)
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

const DIRECTIONS: [Direction; 8] = [
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
pub struct Turn {
    pub arena: Weak<Arena>,
    pub combatant: Weak<Combatant>,
    pub movement: Movement,
}

impl Turn {
    pub fn move_in(&self, direction: Direction) -> bool {
        if !self.movement.any_left() {
            return false;
        }

        let me = self.combatant.upgrade().unwrap();
        match &me.creature.size {
            CreatureSize::Tiny | CreatureSize::Small | CreatureSize::Medium => (),
            _ => todo!("Bigger sizes that take up more than one square are not supported yet..."),
        }

        let Some(new_pos) = me.position.get() + direction else {
            return false;
        };

        let arena = self.arena.upgrade().unwrap();

        let Some(new_sq) = arena.at(new_pos) else {
            return false;
        };

        if new_sq.is_occupied() {
            return false;
        }

        let current_sq = arena.at(me.position.get()).unwrap();

        // Fire the event...

        current_sq.remove_occupant(&me);
        new_sq.add_occupant(&me);

        self.movement
            .left
            .update(|Feet(prev)| Feet(prev - FEET_PER_SQUARE));

        true
    }

    pub fn available_movement_directions(&self) -> [Option<Direction>; 8] {
        if !self.movement.any_left() {
            return [None; 8];
        }

        let arena = self.arena.upgrade().unwrap();
        let combatant = self.combatant.upgrade().unwrap();

        let Some(around) = arena.around(combatant.position.get()) else {
            return [None; 8];
        };

        array::from_fn(|i| {
            around.clockwise[i].and_then(|sq| (!sq.is_occupied()).then_some(DIRECTIONS[i]))
        })
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

    use crate::engine::game::{Dispatcher, Game, combat::arena::Position, creature::Creature};

    // Mainly for Opportunity Attacks
    #[derive(Debug)]
    pub struct PreMoveEvent {
        pub me: Weak<Creature>,
        pub from: Position,
        pub to: Position,
        pub(super) cancelled: Option<()>,
    }

    cancellable!(PreMoveEvent, cancelled);
    register!(PreMoveEvent: dyn EventBase<Game>, register(Identity("MOVE::PRE")));

    #[allow(clippy::unit_arg)]
    impl Event<Game> for PreMoveEvent {
        type Resolved = ();

        fn map_resolved(self) -> impl IntoFuture<Output = Self::Resolved> {
            ready(())
        }

        type Cancelled = ();

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
                let game = Dispatcher::local().await;
                for combatant in game.combat.initiative.iter() {
                    // Skip ourselves.
                    if std::ptr::addr_eq(Rc::as_ptr(combatant), event.me.as_ptr()) {
                        continue;
                    }

                    // TODO: _
                }
            }
        }
    }
}
