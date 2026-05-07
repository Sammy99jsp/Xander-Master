use pyo3::{IntoPyObject, PyErr, PyResult, Python, exceptions::PyStopIteration};
use std::{
    future::ready,
    ops::ControlFlow,
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
            combat::{reaction::AttackOfOpportunity, turn::Turn, win::GameEndReport},
        },
        io::{Agent, DynInterface, agent::IoError, roller::Roller},
    },
    runtime::{
        flow::{Decision, decision::Response},
        futures::{FutureExt, prelude::future::LocalBoxFuture},
    },
};

use crate::{
    api,
    py::{
        coroutine::StoredCoroutine,
        utils::{PythonWeak, UnsafePythonEscape},
    },
};

pub type StopSignal = Arc<AtomicBool>;

#[derive(Debug)]
pub struct PythonInterface {
    pub debug: bool,
}

impl DynInterface for PythonInterface {
    fn log<'a, 'b: 'a>(&'a self, displ: &'b dyn std::fmt::Display) -> LocalBoxFuture<'a, ()> {
        println!("{displ}");
        ready(()).boxed_local()
    }

    fn prompt_dyn<'a>(&'a self, _: Decision) -> LocalBoxFuture<'a, Box<dyn Response>> {
        todo!()
    }

    fn update<'a>(&'a self, game: &'a Game) -> LocalBoxFuture<'a, Result<(), IoError>> {
        async move {
            Python::attach(|py| py.check_signals()).map_err(IoError::new)?;

            if self.debug {
                println!("{}", game.combat.arena.display_debug());

                let current_turn_taker = &game.combat.current_turn().me;
                let initiative = game.combat.initiative();
                for s in initiative {
                    let is_my_turn = std::ptr::eq(Rc::as_ptr(&s), current_turn_taker.as_ptr());
                    println!(
                        "{} {} <{}{} {}/{} [{}]>",
                        if is_my_turn { "*" } else { " " },
                        s.initiative_score.get(),
                        s.creature.name,
                        if s.creature.is_dead() { " ☠" } else { "" },
                        s.creature.stats.health.current(),
                        s.creature.stats.health.max_hp.get().await,
                        s.creature
                            .stats
                            .markers
                            .iter()
                            .map(|m| format!("{m:?}"))
                            .intersperse_with(|| ", ".to_string())
                            .collect::<String>()
                    );
                }
            }
            Ok(())
        }
        .boxed_local()
    }

    fn error(&self, error: IoError) {
        Python::attach(|py| {
            error.downcast_ref::<PyErr>().unwrap().print(py);
        })
    }
}

#[derive(Debug)]
pub struct PythonAgent {
    pub roller: Box<dyn Roller>,
    pub name: String,
    pub coroutine: StoredCoroutine,
    pub game: Weak<Game>,
    pub stop_signal: Arc<AtomicBool>,
}

impl Agent for PythonAgent {
    fn roller(&self) -> &dyn Roller {
        self.roller.as_ref()
    }

    fn game(&self) -> Weak<Game> {
        self.game.clone()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn turn_step(&self, turn: Weak<Turn>) -> LocalBoxFuture<'_, Result<ControlFlow<()>, IoError>> {
        async move {
            python_send(
                self,
                api::turn::Turn {
                    // SAFETY: We must investigate using a semaphore later.
                    turn: unsafe { PythonWeak::new(turn.clone()) },
                    end: Arc::downgrade(&self.stop_signal),
                    game: unsafe { PythonWeak::new(self.game.clone()) },
                    used: false,
                },
            )
            .map_err(IoError::new)?;

            match self.stop_signal.load(Ordering::Relaxed) {
                true => {
                    self.stop_signal.store(false, Ordering::Relaxed);
                    Ok(ControlFlow::Break(()))
                }
                false => Ok(ControlFlow::Continue(())),
            }
        }
        .boxed_local()
    }

    fn opportunity_attack(
        &self,
        attack: Rc<AttackOfOpportunity>,
    ) -> LocalBoxFuture<'_, Result<(), IoError>> {
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
            )
            .map_err(IoError::new)?;

            Ok(())
        }
        .boxed_local()
    }

    fn game_end(&self, report: GameEndReport) -> LocalBoxFuture<'_, Result<(), IoError>> {
        async move {
            let res = python_send(
                self,
                api::game::GameEnd {
                    report: unsafe { UnsafePythonEscape::new(report) },
                    game: unsafe { PythonWeak::new(self.game.clone()) },
                },
            );
            // Handle the case of coroutines returning at the end of the game.
            // This raises StopIteration, which we should handle to prevent everything from crashing.
            match res {
                Err(err) if Python::attach(|py| err.is_instance_of::<PyStopIteration>(py)) => {
                    Ok(())
                }
                Err(err) => Err(IoError::new(err)),
                Ok(()) => Ok(()),
            }
        }
        .boxed_local()
    }
}

#[must_use = "Handle the error!"]
fn python_send<T>(agent: &PythonAgent, value: T) -> PyResult<()>
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
}
