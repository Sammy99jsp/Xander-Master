use pyo3::{IntoPyObject, Python};
use std::{
    any::Any,
    future::ready,
    rc::{Rc, Weak},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use xander::{
    engine::{
        game::{
            Game,
            combat::{reaction::AttackOfOpportunity, turn::Turn},
        },
        io::{Agent, DynInterface, roller::Roller},
    },
    runtime::{
        flow::{Decision, decision::Response},
        futures::{FutureExt, prelude::future::LocalBoxFuture},
    },
};

use crate::{
    api,
    py::{coroutine::StoredCoroutine, utils::PythonWeak},
};

pub type StopSignal = Arc<AtomicBool>;

#[derive(Debug)]
pub struct PythonInterface {}

impl DynInterface for PythonInterface {
    fn log<'a, 'b: 'a>(&'a self, displ: &'b dyn std::fmt::Display) -> LocalBoxFuture<'a, ()> {
        println!("{displ}");
        ready(()).boxed_local()
    }

    fn prompt_dyn<'a>(&'a self, _: Decision) -> LocalBoxFuture<'a, Box<dyn Response>> {
        todo!()
    }

    fn update<'a>(&'a self) -> LocalBoxFuture<'a, Result<(), Box<dyn Any>>> {
        async {
            Python::attach(|py| py.check_signals()).map_err(|err| Box::new(err) as Box<dyn Any>)
        }
        .boxed_local()
    }
}

#[derive(Debug)]
pub struct PythonAgent {
    pub roller: Box<dyn Roller>,
    pub name: String,
    pub coroutine: StoredCoroutine,
    pub game: Weak<Game>,
}

impl Agent for PythonAgent {
    fn roller(&self) -> &dyn Roller {
        self.roller.as_ref()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn turn(&self, turn: Weak<Turn>) -> LocalBoxFuture<'_, Result<(), Box<dyn Any>>> {
        async move {
            let stop_signal = StopSignal::default();

            while !stop_signal.load(Ordering::Relaxed) {
                python_send(
                    self,
                    api::turn::Turn {
                        // SAFETY: We must investigate using a semaphore later.
                        turn: unsafe { PythonWeak::new(turn.clone()) },
                        end: Arc::downgrade(&stop_signal),
                        game: unsafe { PythonWeak::new(self.game.clone()) },
                        used: false,
                    },
                )?;
            }
            Ok(())
        }
        .boxed_local()
    }

    fn opportunity_attack(
        &self,
        attack: Rc<AttackOfOpportunity>,
    ) -> LocalBoxFuture<'_, Result<(), Box<dyn Any>>> {
        async move {
            let stop_signal = StopSignal::default();
            python_send(
                self,
                api::reaction::Reaction {
                    // SAFETY: We must investigate using a semaphore later.
                    kind: api::reaction::ReactionKind::AttackOfOpportunity(
                        api::reaction::AttackOfOpportunity {
                            aoo: unsafe { PythonWeak::new(Rc::downgrade(&attack)) },
                            end: Arc::downgrade(&stop_signal),
                            game: unsafe { PythonWeak::new(self.game.clone()) },
                        },
                    ),
                },
            )?;

            while !stop_signal.load(Ordering::Relaxed) {}
            Ok(())
        }
        .boxed_local()
    }
}

#[must_use = "Handle the error!"]
fn python_send<T>(agent: &PythonAgent, value: T) -> Result<(), Box<dyn Any>>
where
    T: for<'py> IntoPyObject<'py>,
{
    Python::attach(move |py| {
        let coroutine = agent.coroutine.bind(py);

        // I'm using Option<_> here just to allow for None
        // Being returned...
        coroutine.send::<Option<u8>, _>(value)
    })
    .map(|_| ())
    .map_err(|a| Box::new(a) as Box<dyn Any>)
}
