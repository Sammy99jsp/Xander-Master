use std::rc::Rc;

use xander_runtime::flow::Dispatcher;

use crate::engine::game::combat::Combat;

pub mod combat;
pub mod creature;
pub mod health;
pub mod stats;

#[derive(Debug)]
pub struct Game {
    pub combat: Combat,
    pub dispatcher: Rc<Dispatcher<Self>>,
}

impl Game {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            combat: Combat::new(),
            // SAFETY: Using Rc::new_cyclic to ensure lifetimes satisfy the Dispatcher.
            dispatcher: unsafe { Dispatcher::new(this.clone()) },
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::game::{Game, creature};

    #[test]
    fn new_game() {
        smol::block_on(async {
            let game = Game::new();

            let creature = creature::test_creature();
            game.dispatcher
                .dispatch(async {
                    let b = creature.stats.proficiency_bonus.get().await;
                    println!("{b:?}");
                })
                .await;
        })
    }
}
