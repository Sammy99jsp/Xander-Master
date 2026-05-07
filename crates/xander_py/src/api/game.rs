use std::{
    rc::Rc,
    sync::{Arc, atomic::AtomicBool},
};

use numpy::{IntoPyArray, ndarray};
use pyo3::{
    exceptions::{PyTypeError, PyValueError},
    prelude::*,
};
use xander::{
    d20::rand::{self, Rng, SeedableRng},
    engine::{game::combat::arena::Dimensions, io::roller::Roller},
    runtime::smol,
};

use crate::{
    api::turn::{Combatant, View},
    py::{
        coroutine::{Coroutine, StoredCoroutine},
        io::{PythonAgent, PythonInterface},
        utils::{
            MaybeStrong, OrExpired, PythonOwnedRc, PythonWeak, UnsafePythonEscape, run_future,
        },
    },
};

mod rs {
    pub use xander::engine::game::{
        Game,
        combat::{
            Combatant,
            arena::{Arena, Position},
            win::GameEndReport,
        },
        creature::Creature,
        measure::{FEET_PER_SQUARE, Feet, Squares},
    };
}

#[pyclass]
#[derive(PartialEq, Eq)]
pub struct Position {
    x: u32,
    y: u32,
}

#[pymethods]
impl Position {
    #[pyo3(name = "__eq__")]
    pub fn _eq(&self, rhs: &Self) -> bool {
        self.x == rhs.x && self.y == rhs.y
    }
}

#[pyclass]
#[derive(FromPyObject)]
pub struct Arena {
    width: u32,
    height: u32,
}

#[pymethods]
impl Arena {
    #[new]
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    #[getter]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[getter]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[pyo3(name = "__repr__")]
    pub fn repr(&self) -> String {
        format!("Arena({} x {})", self.width, self.height)
    }

    #[pyo3(signature = (*, seed = None))]
    pub fn random_square(&self, seed: Option<u64>) -> Position {
        let w = self.width / rs::FEET_PER_SQUARE;
        let h = self.height / rs::FEET_PER_SQUARE;

        let mut rng = match seed {
            Some(seed) => rand::rngs::Xoshiro128PlusPlus::seed_from_u64(seed),
            None => rand::rngs::Xoshiro128PlusPlus::from_rng(&mut rand::rng()),
        };

        Position {
            x: rng.next_u32() % w,
            y: rng.next_u32() % h,
        }
    }

    pub fn square_at(&self, x: u32, y: u32) -> Position {
        let (x, y) = (x / rs::FEET_PER_SQUARE, y / rs::FEET_PER_SQUARE);
        Position { x, y }
    }
}

#[pyclass]
pub struct Game {
    pub game: PythonOwnedRc<rs::Game>,
}

#[pymethods]
impl Game {
    #[new]
    #[allow(clippy::new_without_default)]
    #[pyo3(signature = (arena, debug = false))]
    pub fn new(arena: Arena, debug: bool) -> PyResult<Self> {
        Ok(Self {
            game: unsafe {
                PythonOwnedRc::new({
                    rs::Game::new(
                        PythonInterface { debug },
                        rs::Arena::new_feet(Dimensions {
                            width: rs::Feet(arena.width),
                            height: rs::Feet(arena.height),
                        })
                        .ok_or_else(|| {
                            PyValueError::new_err("Arena dimensions must be multiples of five")
                        })?,
                    )
                })
            },
        })
    }

    pub fn join<'py>(
        &mut self,
        mut agent: PyRefMut<'py, Agent>,
        mut creature: PyRefMut<'py, Creature>,
        position: PyRef<'py, Position>,
    ) -> PyResult<Me> {
        let creature = creature.creature.take_strong().unwrap();
        let game = PythonOwnedRc::into_inner(self.game.clone());
        let actor = self.game.interface.add_actor(PythonAgent {
            roller: agent.roller.take().unwrap(),
            name: agent.name.clone(),
            coroutine: agent.coroutine.clone_ref(agent.py()),
            game: PythonOwnedRc::downgrade(&self.game),
            stop_signal: Arc::new(AtomicBool::new(false)),
        });

        let combatant = run_future(
            game,
            self.game.combat.enroll(
                creature,
                actor,
                rs::Position {
                    x: rs::Squares(position.x),
                    y: rs::Squares(position.y),
                },
            ),
        )
        .map_err(|err| PyValueError::new_err(err.to_string()))?;

        Ok(Me {
            me: unsafe { PythonWeak::new(combatant) },
            game: unsafe { PythonWeak::new(PythonOwnedRc::downgrade(&self.game)) },
        })
    }

    pub fn start<'py>(&mut self, py: Python<'py>) -> PyResult<()> {
        py.detach(|| {
            let result = smol::block_on(self.game.combat.start(&self.game).into_future());
            match &result {
                Ok(()) => return Ok(()),
                Err(err) if err.is::<PyErr>() => (),
                Err(err) => {
                    panic!("Cannot deal with this error of type: {}", err.type_name())
                }
            }

            Err(result
                .unwrap_err()
                .downcast::<PyErr>()
                .unwrap_or_else(|_| panic!("Already checked!")))
        })
    }
}

#[pyclass]
pub struct Agent {
    name: String,
    roller: Option<Box<dyn Roller>>,
    coroutine: StoredCoroutine,
}

#[pymethods]
impl Agent {
    #[new]
    #[pyo3(signature = (name, coroutine, *, seed = None))]
    #[allow(clippy::new_without_default)]
    pub fn new<'py>(
        name: String,
        coroutine: Bound<'py, PyAny>,
        seed: Option<Bound<'py, PyAny>>,
    ) -> PyResult<Self> {
        let coroutine = Coroutine::new(coroutine)?.unbind();

        let roller = match seed {
            Some(seed) => match seed.as_borrowed() {
                s if let Ok(random) = s.extract::<String>()
                    && random == "random" =>
                {
                    xander::d20::provider::local_rng::LocalRng::new()
                }
                s if let Ok(int) = s.extract::<u64>() => {
                    xander::d20::provider::local_rng::LocalRng::with_seed(int)
                }
                _ => {
                    return Err(PyTypeError::new_err(
                        "Expected either \"random\" or a positive integer for `seed` argument.",
                    ));
                }
            },
            None => {
                // TODO: Convert this to a more formal Python warning or something...
                eprintln!(
                    "[Xander] You are using a random seed for RNG by default.\n\tIt is advised to instead use a fixed seed with `Agent(..., seed=<int>)` for reproducibility, or explicitly use `Agent(..., seed=\"random\")`."
                );

                xander::d20::provider::local_rng::LocalRng::new()
            }
        };

        Ok(Self {
            name,
            coroutine,
            roller: Some(Box::new(roller)),
        })
    }
}

#[pyclass]
pub struct Me {
    pub me: PythonWeak<rs::Combatant>,
    pub game: PythonWeak<rs::Game>,
}

impl Me {
    pub fn upgrade(&self) -> PyResult<Rc<rs::Combatant>> {
        self.me.upgrade_or_expired("Combatant")
    }
}

#[pymethods]
impl Me {
    #[getter]
    pub fn name(&self) -> PyResult<String> {
        Ok(self.upgrade()?.creature.name.clone())
    }

    #[getter]
    pub fn hp<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, numpy::PyArray1<f32>>> {
        let combatant = self.upgrade()?;
        let game = self.game.upgrade_or_expired("Combat")?;

        let current_hp = combatant.creature.stats.health.current();
        let max_hp = run_future(game, combatant.creature.stats.health.max_hp.get());
        Ok(ndarray::Array1::from_iter([current_hp as f32, max_hp as f32]).into_pyarray(py))
    }

    #[getter]
    pub fn position<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, numpy::PyArray1<f32>>> {
        let pos = self.upgrade()?.position.get();
        Ok(ndarray::Array1::from_iter([pos.x.0 as f32, pos.y.0 as f32]).into_pyarray(py))
    }

    #[getter]
    pub fn view(&self) -> PyResult<View> {
        let game = self.game.upgrade_or_expired("Game")?;
        let me = self.me.upgrade_or_expired("Combat")?;
        let view = run_future(game, me.view());

        Ok(View {
            game: self.game.clone(),
            me: unsafe { PythonWeak::new(view.me.clone()) },
            allies: unsafe {
                UnsafePythonEscape::new(
                    view.allies.into_iter().map(|c| Rc::downgrade(&c)).collect(),
                )
            },
            enemies: unsafe {
                UnsafePythonEscape::new(
                    view.enemies
                        .into_iter()
                        .map(|c| Rc::downgrade(&c))
                        .collect(),
                )
            },
        })
    }

    #[pyo3(name = "__repr__")]
    pub fn repr(&self) -> PyResult<String> {
        let name = self.name()?;
        let combatant = self.upgrade()?;
        let game = self.game.upgrade_or_expired("Combat")?;

        let current_hp = combatant.creature.stats.health.current();
        let max_hp = run_future(game, combatant.creature.stats.health.max_hp.get());

        Ok(format!("{name} <{current_hp}/{max_hp}>"))
    }

    pub fn distance_from<'py>(&self, combatant: PyRef<'py, Combatant>) -> PyResult<u32> {
        let me = self.upgrade()?;
        let combatant = combatant.upgrade()?;

        Ok(me.distance_to(&combatant).0)
    }

    pub fn displacement_from<'py>(
        &self,
        py: Python<'py>,
        combatant: PyRef<'py, Combatant>,
    ) -> PyResult<Bound<'py, numpy::PyArray1<f32>>> {
        let me = self.upgrade()?;
        let combatant = combatant.upgrade()?;

        let displacement = me.position.get().displacement_to(combatant.position.get());
        Ok(
            ndarray::Array1::<f32>::from_vec(vec![
                displacement.x.0 as f32,
                displacement.y.0 as f32,
            ])
            .into_pyarray(py),
        )
    }

    #[getter]
    pub fn creature(&self) -> PyResult<Creature> {
        Ok(Creature {
            creature: MaybeStrong::Weak(Rc::downgrade(&self.upgrade()?.creature)),
            game: self.game.clone(),
        })
    }

    #[getter]
    pub fn len_actions(&self) -> PyResult<usize> {
        let game = self.game.upgrade_or_expired("Game")?;
        let me = self.upgrade()?;
        let iter = run_future(game, me.actions());
        Ok(iter.count())
    }
}

#[pyclass]
pub struct Creature {
    pub creature: MaybeStrong<rs::Creature>,
    pub game: PythonWeak<rs::Game>,
}

#[pymethods]
impl Creature {}

#[pyclass]
pub struct GameEnd {
    pub report: UnsafePythonEscape<rs::GameEndReport>,
    pub game: PythonWeak<rs::Game>,
}

#[pymethods]
impl GameEnd {
    #[getter]
    pub fn won(&self) -> bool {
        self.report.won
    }

    #[getter]
    pub fn me(&self) -> Me {
        Me {
            me: unsafe { PythonWeak::new(self.report.me.clone()) },
            game: self.game.clone(),
        }
    }
}
