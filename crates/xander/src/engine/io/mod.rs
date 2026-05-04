pub mod agent;
pub mod roller;

use std::{
    cell::{Cell, RefCell},
    future::ready,
    rc::Rc,
};

use d20::provider::local_rng::LocalRng;
use smol::future::FutureExt;
use xander_runtime::{
    flow::{Interface as FlowInterface, io::prelude::*},
    futures::prelude::future::LocalBoxFuture,
};

pub use agent::Agent;

use crate::engine::io::agent::NoopAgent;

#[derive(Debug)]
pub struct Interface {
    current_index: Cell<u32>,
    actors: RefCell<Vec<Rc<dyn Agent>>>,
    base: Box<dyn DynInterface>,
}

impl Interface {
    pub fn new<Io>(base: Io) -> Self
    where
        Io: DynInterface + 'static,
    {
        Self {
            actors: RefCell::new(vec![Rc::new(NoopAgent {
                name: "GM".to_string(),
                roller: Box::new(LocalRng::with_seed(0)),
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
}

impl FlowInterface for Interface {
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
    fn log<'a, 'b: 'a>(&'a self, displ: &'b dyn std::fmt::Display) -> LocalBoxFuture<'a, ()>;
    fn prompt_dyn<'a>(&'a self, decision: Decision) -> LocalBoxFuture<'a, Box<dyn Response>>;
    fn update<'a>(&'a self) -> LocalBoxFuture<'a, Result<(), Box<dyn Any>>>;
}

impl<T> DynInterface for T
where
    T: FlowInterface + ?Sized,
{
    #[inline]
    fn log<'a, 'b: 'a>(&'a self, displ: &'b dyn std::fmt::Display) -> LocalBoxFuture<'a, ()> {
        FlowInterface::log(self, displ)
    }

    #[inline]
    fn prompt_dyn<'a>(&'a self, decision: Decision) -> LocalBoxFuture<'a, Box<dyn Response>> {
        FlowInterface::prompt_dyn(self, decision)
    }

    fn update<'a>(&'a self) -> LocalBoxFuture<'a, Result<(), Box<dyn Any>>> {
        ready(Ok(())).boxed_local()
    }
}
