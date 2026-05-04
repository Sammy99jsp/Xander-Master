use std::{any::Any, rc::Rc};

use xander_runtime::flow::{
    Dispatcher as FlowDispatcher,
    dispatcher::DispatchState,
    event::{EventHandler, Outcome},
    io::Interface as FlowInterface,
};

use crate::engine::{
    game::{
        combat::{Combat, arena::Arena},
        flow::EventHandlers,
    },
    io::{DynInterface, Interface},
};

pub mod combat;
pub mod creature;
pub mod flow;
pub mod health;
pub mod magic;
pub mod measure;
pub mod stats;

pub type Dispatcher = FlowDispatcher<Game>;

#[derive(Debug)]
pub struct Game {
    pub combat: Rc<Combat>,
    pub dispatcher: Rc<Dispatcher>,
    pub interface: Interface,
    pub event_handlers: EventHandlers,
}

impl Game {
    pub fn new<Io>(base: Io, arena: Arena) -> Rc<Self>
    where
        Io: FlowInterface + 'static,
    {
        Rc::new_cyclic(|this| Self {
            combat: Rc::new(Combat::new(arena)),
            // SAFETY: Using Rc::new_cyclic to ensure lifetimes satisfy the Dispatcher.
            dispatcher: unsafe { Dispatcher::new(this.clone()) },
            interface: Interface::new(base),
            event_handlers: EventHandlers::new(),
        })
    }
}

impl DispatchState for Game {
    type Interface = Interface;

    fn interface(&self) -> &Self::Interface {
        &self.interface
    }

    fn handle<E: xander_runtime::flow::Event<Self>>(
        &self,
        event: E,
    ) -> impl IntoFuture<Output = Outcome<Self, E>> {
        self.event_handlers.handle(event)
    }

    fn listen<H>(&self, handler: H)
    where
        H: EventHandler<Self> + 'static,
    {
        self.event_handlers.listen(handler);
    }

    fn update(&self) -> impl IntoFuture<Output = Result<(), Box<dyn Any>>> {
        self.interface.update()
    }
}

#[cfg(test)]
mod tests {
    use xander_runtime::flow::io::TestInterface;

    use crate::engine::game::{
        Game,
        combat::arena::Arena,
        creature,
        stats::skill::{Skill, profs::SkillProficiency},
    };

    #[test]
    fn new_game() {
        let game = Game::new(TestInterface, Arena::test());

        let combatant = creature::test_combatant();
        combatant
            .creature
            .stats
            .proficiencies
            .insert(SkillProficiency {
                skill: Skill::Acrobatics,
            });

        let pinned =
            smol::block_on(smol::future::poll_once(game.dispatcher.dispatch(async {
                combatant.creature.stats.proficiency_bonus.get().await
            })));
        println!("{pinned:?}")
    }
}
