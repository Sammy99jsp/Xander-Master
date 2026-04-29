pub use crate::engine::game::Game;

pub type Outcome<E> = xander_runtime::flow::event::Outcome<Game, E>;

pub use xander_runtime::{
    flow::event::{Event, EventBase, EventHandler, EventHandlerBase},
    identity,
};
