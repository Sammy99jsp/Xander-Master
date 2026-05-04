use std::{
    any::Any,
    future::{self, ready},
    rc::{Rc, Weak},
    task::Poll,
};

use xander_runtime::futures::{FutureExt, future::LocalBoxFuture};

use crate::engine::{
    game::combat::{reaction::AttackOfOpportunity, turn::Turn},
    io::roller::Roller,
};

pub trait Agent: std::fmt::Debug {
    fn roller(&self) -> &dyn Roller;
    fn name(&self) -> &str;

    #[must_use]
    fn turn(&self, turn: Weak<Turn>) -> LocalBoxFuture<'_, Result<(), Box<dyn Any>>>;

    #[must_use]
    fn opportunity_attack(
        &self,
        attack: Rc<AttackOfOpportunity>,
    ) -> LocalBoxFuture<'_, Result<(), Box<dyn Any>>>;
}

#[derive(Debug)]
pub struct NoopAgent {
    pub name: String,
    pub roller: Box<dyn Roller>,
}

impl Agent for NoopAgent {
    fn roller(&self) -> &dyn Roller {
        self.roller.as_ref()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn turn(&self, turn: Weak<Turn>) -> LocalBoxFuture<'_, Result<(), Box<dyn Any>>> {
        future::poll_fn(move |_| {
            println!("No-op got turn {:?}", turn.upgrade().unwrap());
            Poll::Ready(Ok(()))
        })
        .boxed_local()
    }

    fn opportunity_attack(
        &self,
        _: Rc<AttackOfOpportunity>,
    ) -> LocalBoxFuture<'_, Result<(), Box<dyn Any>>> {
        ready(Ok(())).boxed_local()
    }
}
