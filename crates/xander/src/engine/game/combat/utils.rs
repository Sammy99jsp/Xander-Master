use std::rc::{Rc, Weak};

use xander_runtime::Lived;

use crate::engine::game::{Dispatcher, combat::Combat, creature::Creature, measure::time::Rounds};

#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct NextTurn {
    pub start: Rounds,
    pub me: Weak<Creature>,
    pub combat: Weak<Combat>,
}

impl NextTurn {
    pub async fn new(me: &Rc<Creature>) -> Self {
        let game = Dispatcher::local().await;

        Self {
            start: game.combat.clock.rounds(),
            me: Rc::downgrade(me),
            combat: Rc::downgrade(&game.combat),
        }
    }

    pub fn yet(&self) -> bool {
        use std::cmp::Ordering::*;

        let Some(combat): Option<std::rc::Rc<Combat>> = self.combat.upgrade() else {
            return false;
        };

        let round_cmp = combat.clock.rounds().cmp(&self.start);
        let turn_order_cmp = combat
            .turn_order_of(&self.me)
            .cmp(&combat.clock.current_turn_order());

        match (round_cmp, turn_order_cmp) {
            (Less, _) | (Equal, Less) => unreachable!("Combat clock only goes forward"),

            // Not our next turn yet.
            (Equal, Equal | Greater) | (Greater, Less) => false,

            // Exactly our turn.
            (Greater, Equal) => true,

            // After our next turn.
            (Greater, Greater) => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UntilNextTurn(pub NextTurn);

impl Lived for UntilNextTurn {
    fn is_alive(&self) -> bool {
        !self.0.yet()
    }
}

#[derive(Debug, Clone)]
pub struct Availability<T> {
    value: T,
    available: bool,
}

impl<T> Availability<T> {
    pub const fn available(value: T) -> Self {
        Self {
            value,
            available: true,
        }
    }

    pub const fn unavailable(value: T) -> Self {
        Self {
            value,
            available: false,
        }
    }

    pub const fn is_available(&self) -> bool {
        self.available
    }

    pub fn map<U, F>(self, f: F) -> Availability<U>
    where
        F: FnOnce(T) -> U,
    {
        Availability {
            value: f(self.value),
            available: self.available,
        }
    }

    pub fn and<F>(self, f: F) -> Self
    where
        F: for<'a> FnOnce(&'a T) -> bool,
    {
        Self {
            available: self.available && f(&self.value),
            value: self.value,
        }
    }

    pub fn as_ref(&self) -> Availability<&T> {
        Availability {
            value: &self.value,
            available: self.available,
        }
    }

    pub fn value(self) -> T {
        self.value
    }
}

impl<T, E> Availability<Result<T, E>> {
    pub fn transpose(self) -> Result<Availability<T>, E> {
        match self.value {
            Ok(value) => Ok(Availability {
                value,
                available: self.available,
            }),
            Err(err) => Err(err),
        }
    }
}
