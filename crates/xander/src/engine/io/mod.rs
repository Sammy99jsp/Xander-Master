pub mod roller;

use std::collections::BTreeMap;

use d20::provider::local_rng::LocalRng;
use xander_runtime::{
    flow::{Interface as FlowInterface, io::prelude::*},
    futures::prelude::future::LocalBoxFuture,
};

use crate::engine::io::roller::Roller;

#[derive(Debug)]
pub struct User {
    pub name: String,
    pub roller: Box<dyn Roller>,
}

#[derive(Debug)]
pub struct Interface {
    current_index: u32,
    actors: BTreeMap<Actor, User>,
    base: Box<dyn DynInterface>,
}

impl Interface {
    pub fn new<Io>(base: Io) -> Self
    where
        Io: FlowInterface + 'static,
    {
        Self {
            actors: {
                let mut map = BTreeMap::new();
                map.insert(
                    Actor::GM,
                    User {
                        name: "GM".to_string(),
                        roller: Box::new(LocalRng::new(0)),
                    },
                );
                map
            },
            current_index: 1,
            base: Box::new(base),
        }
    }

    pub fn add_actor(&mut self, user: User) -> Actor {
        // SAFETY: This will be unique since we are incrementing.
        let new = unsafe { Actor::new(self.current_index + 1) };
        self.current_index += 1;

        self.actors.insert(new, user);

        new
    }
}

impl FlowInterface for Interface {
    type ActorState = User;

    fn log<'a, 'b: 'a>(&'a self, displ: &'b dyn std::fmt::Display) -> LocalBoxFuture<'a, ()> {
        self.base.log(displ)
    }

    fn prompt_dyn<'a>(&'a self, decision: Decision) -> LocalBoxFuture<'a, Box<dyn Response>> {
        self.base.prompt_dyn(decision)
    }

    fn state_for(&self, actor: Actor) -> &Self::ActorState {
        // SAFETY: Only we can issue new Actors, so there should always be a corresponding entry.
        unsafe { self.actors.get(&actor).unwrap_unchecked() }
    }
}

pub trait DynInterface: std::fmt::Debug {
    fn log<'a, 'b: 'a>(&'a self, displ: &'b dyn std::fmt::Display) -> LocalBoxFuture<'a, ()>;
    fn prompt_dyn<'a>(&'a self, decision: Decision) -> LocalBoxFuture<'a, Box<dyn Response>>;
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
}
