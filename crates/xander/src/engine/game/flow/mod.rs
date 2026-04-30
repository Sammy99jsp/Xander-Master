use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use dynx::Identity;
use xander_runtime::{
    flow::{
        Event,
        event::{EventHandler, EventHandlerBase, Outcome},
    },
    lived::LivedList,
};

use crate::engine::game::Game;

type Table = BTreeMap<&'static str, LivedList<Rc<dyn EventHandlerBase<Game>>>>;

#[derive(Debug)]
pub struct EventHandlers {
    inner: RefCell<Table>,
}

impl EventHandlers {
    pub const fn new() -> Self {
        Self {
            inner: RefCell::new(BTreeMap::new()),
        }
    }

    pub fn listen<H>(&self, handler: H)
    where
        H: EventHandler<Game> + 'static,
    {
        let mut inner = self.inner.borrow_mut();
        inner
            .entry(<H::Event as Identity>::LOCAL_ID)
            .or_default()
            .get_mut()
            .push(Rc::new(handler));
    }

    pub async fn handle<E>(&self, mut event: E) -> Outcome<Game, E>
    where
        E: Event<Game>,
    {
        let handlers = {
            let inner = self.inner.borrow();
            inner
                .get(E::LOCAL_ID)
                .map(|handlers| handlers.read().iter().cloned().collect::<Vec<_>>())
        };

        let Some(handlers) = handlers else {
            return event.finalize().await;
        };

        for handler in handlers {
            handler.handle(&mut event).await;

            if event.is_cancelled() {
                break;
            }
        }

        event.finalize().await
    }
}

impl Default for EventHandlers {
    fn default() -> Self {
        Self::new()
    }
}
