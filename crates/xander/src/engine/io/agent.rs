use std::{
    any::Any,
    future::ready,
    ops::ControlFlow,
    rc::{Rc, Weak},
};

use xander_runtime::{
    flow::dispatcher::DispatchState,
    futures::{FutureExt, future::LocalBoxFuture},
};

use crate::engine::{
    game::{
        Game,
        combat::{reaction::AttackOfOpportunity, turn::Turn, win::GameEndReport},
        creature::Creature,
    },
    io::roller::Roller,
};

pub struct IoError(Box<dyn Any>);

impl IoError {
    pub fn new<T: 'static>(error: T) -> Self {
        Self(Box::new(error))
    }

    pub fn downcast<T: 'static>(self) -> Result<T, Self> {
        self.0.downcast::<T>().map(|inner| *inner).map_err(Self)
    }
    pub fn is<T: 'static>(&self) -> bool {
        self.0.is::<T>()
    }
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.downcast_ref::<T>()
    }

    pub fn type_name(&self) -> &'static str {
        std::any::type_name_of_val(self.0.as_ref())
    }
}

pub trait AgentExt: Agent {
    fn turn(&self, turn: Weak<Turn>) -> impl IntoFuture<Output = Result<(), IoError>>;
    fn opportunity_attack(
        &self,
        attack: Rc<AttackOfOpportunity>,
    ) -> LocalBoxFuture<'_, Result<(), IoError>>;
}

impl<T: Agent + ?Sized> AgentExt for T {
    fn turn(&self, turn: Weak<Turn>) -> impl IntoFuture<Output = Result<(), IoError>> {
        async move {
            let me: &Rc<Creature> = &turn.upgrade().unwrap().me.upgrade().unwrap().creature;
            loop {
                let game: Rc<Game> = self.game().upgrade().unwrap();

                if game.combat.is_terminating() {
                    break;
                }

                game.update().await?;

                if !me.can_take_turns() {
                    break;
                }

                match self.turn_step(turn.clone()).await? {
                    ControlFlow::Continue(()) => continue,
                    ControlFlow::Break(_) => break,
                }
            }

            Ok(())
        }
    }

    fn opportunity_attack(
        &self,
        attack: Rc<AttackOfOpportunity>,
    ) -> LocalBoxFuture<'_, Result<(), IoError>> {
        async move {
            self.game().upgrade().unwrap().update().await?;
            Agent::opportunity_attack(self, attack).await
        }
        .boxed_local()
    }
}

impl Agent for Rc<dyn Agent> {
    fn roller(&self) -> &dyn Roller {
        self.as_ref().roller()
    }

    fn name(&self) -> &str {
        self.as_ref().name()
    }

    fn game(&self) -> Weak<Game> {
        self.as_ref().game()
    }

    fn turn_step(&self, turn: Weak<Turn>) -> LocalBoxFuture<'_, Result<ControlFlow<()>, IoError>> {
        self.as_ref().turn_step(turn)
    }

    fn opportunity_attack(
        &self,
        attack: Rc<AttackOfOpportunity>,
    ) -> LocalBoxFuture<'_, Result<(), IoError>> {
        self.as_ref().opportunity_attack(attack)
    }

    fn game_end(&self, report: GameEndReport) -> LocalBoxFuture<'_, Result<(), IoError>> {
        self.as_ref().game_end(report)
    }
}

pub trait Agent: std::fmt::Debug {
    fn roller(&self) -> &dyn Roller;
    fn name(&self) -> &str;

    fn game(&self) -> Weak<Game>;

    #[must_use]
    fn turn_step(&self, turn: Weak<Turn>) -> LocalBoxFuture<'_, Result<ControlFlow<()>, IoError>>;

    #[must_use]
    fn opportunity_attack(
        &self,
        attack: Rc<AttackOfOpportunity>,
    ) -> LocalBoxFuture<'_, Result<(), IoError>>;

    #[must_use]
    fn game_end(&self, report: GameEndReport) -> LocalBoxFuture<'_, Result<(), IoError>>;
}

#[derive(Debug)]
pub struct NoopAgent {
    pub name: String,
    pub roller: Box<dyn Roller>,
    pub game: Weak<Game>,
}

impl Agent for NoopAgent {
    fn game(&self) -> Weak<Game> {
        self.game.clone()
    }

    fn roller(&self) -> &dyn Roller {
        self.roller.as_ref()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn turn_step(&self, _: Weak<Turn>) -> LocalBoxFuture<'_, Result<ControlFlow<()>, IoError>> {
        ready(Ok(ControlFlow::Break(()))).boxed_local()
    }

    fn opportunity_attack(
        &self,
        _: Rc<AttackOfOpportunity>,
    ) -> LocalBoxFuture<'_, Result<(), IoError>> {
        ready(Ok(())).boxed_local()
    }

    fn game_end(&self, _: GameEndReport) -> LocalBoxFuture<'_, Result<(), IoError>> {
        ready(Ok(())).boxed_local()
    }
}
