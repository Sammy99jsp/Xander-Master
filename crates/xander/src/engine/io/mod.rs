pub mod agent;
pub mod roller;

use std::{
    cell::{Cell, RefCell},
    future::ready,
    rc::{Rc, Weak},
};

use d20::provider::local_rng::LocalRng;
use smol::future::FutureExt;
use xander_runtime::{
    flow::{
        Interface as FlowInterface,
        io::{TestInterface, prelude::*},
    },
    futures::prelude::future::LocalBoxFuture,
};

pub use agent::Agent;

use crate::engine::{
    game::Game,
    io::agent::{IoError, NoopAgent},
};

#[derive(Debug)]
pub struct Interface {
    current_index: Cell<u32>,
    actors: RefCell<Vec<Rc<dyn Agent>>>,
    base: Box<dyn DynInterface>,
}

impl Interface {
    pub fn new<Io>(base: Io, game: Weak<Game>) -> Self
    where
        Io: DynInterface + 'static,
    {
        Self {
            actors: RefCell::new(vec![Rc::new(NoopAgent {
                name: "GM".to_string(),
                roller: Box::new(LocalRng::with_seed(0)),
                game,
            })]),
            current_index: Cell::new(1),
            base: Box::new(base),
        }
    }

    pub fn add_actor<A>(&self, agent: A) -> Actor
    where
        A: Agent + 'static,
    {
        // SAFETY: This will be unique since we are incrementing.
        let new = unsafe { Actor::new(self.current_index.get()) };
        self.current_index.update(|i| i + 1);

        self.actors.borrow_mut().push(Rc::new(agent));

        new
    }

    pub fn log_error(&self, error: IoError) {
        self.base.error(error);
    }
}

impl FlowInterface for Interface {
    type IoError = IoError;
    type ActorState = dyn Agent;

    fn log<'a, 'b: 'a>(&'a self, displ: &'b dyn std::fmt::Display) -> LocalBoxFuture<'a, ()> {
        self.base.log(displ)
    }

    fn prompt_dyn<'a>(&'a self, decision: Decision) -> LocalBoxFuture<'a, Box<dyn Response>> {
        self.base.prompt_dyn(decision)
    }

    fn state_for(&self, actor: Actor) -> Rc<Self::ActorState> {
        let actors = self.actors.borrow();
        actors
            .get(actor.as_index())
            .expect("Actor to exist at index")
            .clone()
    }
}

pub trait DynInterface: std::fmt::Debug {
    fn error(&self, error: IoError);
    fn log<'a, 'b: 'a>(&'a self, displ: &'b dyn std::fmt::Display) -> LocalBoxFuture<'a, ()>;
    fn prompt_dyn<'a>(&'a self, decision: Decision) -> LocalBoxFuture<'a, Box<dyn Response>>;
    fn update<'a>(&'a self, game: &'a Game) -> LocalBoxFuture<'a, Result<(), IoError>>;
}

impl DynInterface for Interface {
    #[inline]
    fn log<'a, 'b: 'a>(&'a self, displ: &'b dyn std::fmt::Display) -> LocalBoxFuture<'a, ()> {
        self.base.log(displ)
    }

    #[inline]
    fn prompt_dyn<'a>(&'a self, decision: Decision) -> LocalBoxFuture<'a, Box<dyn Response>> {
        self.base.prompt_dyn(decision)
    }

    fn update<'a>(&'a self, game: &'a Game) -> LocalBoxFuture<'a, Result<(), IoError>> {
        self.base.update(game)
    }

    fn error(&self, error: IoError) {
        self.base.error(error);
    }
}

impl DynInterface for TestInterface {
    fn log<'a, 'b: 'a>(&'a self, displ: &'b dyn std::fmt::Display) -> LocalBoxFuture<'a, ()> {
        println!("{displ}");
        ready(()).boxed_local()
    }

    fn prompt_dyn<'a>(&'a self, _: Decision) -> LocalBoxFuture<'a, Box<dyn Response>> {
        unimplemented!()
    }

    fn update<'a>(&'a self, _: &'a Game) -> LocalBoxFuture<'a, Result<(), IoError>> {
        ready(Ok(())).boxed_local()
    }

    fn error(&self, _: IoError) {}
}
