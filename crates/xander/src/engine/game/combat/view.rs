use std::rc::{Rc, Weak};

use crate::engine::game::{Dispatcher, combat::Combatant};

#[derive(Debug)]
pub struct View<'a> {
    pub me: Weak<Combatant>,
    pub allies: Vec<Rc<Combatant>>,
    pub enemies: Vec<Rc<Combatant>>,
    __: &'a (),
}

impl<'a> View<'a> {
    pub(super) async fn new(me: &Rc<Combatant>) -> Self {
        let game = Dispatcher::local().await;

        let (allies, enemies): (Vec<_>, Vec<_>) = game
            .combat
            .initiative()
            .into_iter()
            .filter(|c| !Rc::ptr_eq(c, me))
            .partition(|c| c.affiliation.is_friendly(&me.affiliation));

        Self {
            me: Rc::downgrade(me),
            allies,
            enemies,
            __: &(),
        }
    }
}
