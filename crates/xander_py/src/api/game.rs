use std::rc::Rc;

use pyo3::{
    exceptions::{PyIOError, PyValueError},
    prelude::*,
    types::PyTuple,
};
use xander::{
    d20::rand::Rng,
    engine::{
        game::combat::arena::Dimensions,
        io::roller::Roller,
        json::{self, serde_json},
    },
    runtime::smol,
};

use crate::py::{
    coroutine::{Coroutine, StoredCoroutine},
    io::{PythonAgent, PythonInterface},
    utils::{MaybeStrong, OrExpired, PyFile, PythonOwnedRc, PythonWeak, run_future},
};

mod rs {
    pub use xander::engine::{
        game::{
            Game,
            combat::{
                Combatant,
                arena::{Arena, Position},
                win::GameEndReport,
            },
            creature::Creature,
            measure::{FEET_PER_SQUARE, Squares},
        },
        io::Interface,
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

    pub fn random_square(&self) -> Position {
        let w = self.width / rs::FEET_PER_SQUARE;
        let h = self.height / rs::FEET_PER_SQUARE;

        let mut rng = xander::d20::rand::rng();

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
    pub fn new(arena: Arena) -> Self {
        Self {
            game: unsafe {
                PythonOwnedRc::new(rs::Game::new(
                    rs::Interface::new(PythonInterface {}),
                    rs::Arena::new(Dimensions {
                        width: rs::Squares(arena.width / rs::FEET_PER_SQUARE),
                        height: rs::Squares(arena.height / rs::FEET_PER_SQUARE),
                    }),
                ))
            },
        }
    }

    #[pyo3(signature = (*args, name=None))]
    pub fn load_creature_json<'py>(
        &self,
        args: &Bound<'py, PyTuple>,
        name: Option<String>,
    ) -> PyResult<Creature> {
        if args.len() == 0 {
            return Err(PyValueError::new_err(
                "Expected either a file-like or file path",
            ));
        }

        let file = PyFile::from_str_or_file(&args.get_item(0)?, false)?;
        let mut raw = serde_json::from_reader::<_, json::creature::Creature>(file.0)
            .map_err(|err| PyIOError::new_err(err.to_string()))?;

        // Customize name here
        if let Some(name) = name {
            raw.name = name;
        }

        let creature = self.game.combat.load_raw_character(raw);

        unsafe {
            Ok(Creature {
                creature: MaybeStrong::strong(creature),
                game: PythonWeak::new(PythonOwnedRc::downgrade(&self.game)),
            })
        }
    }

    pub fn join<'py>(
        &mut self,
        mut agent: PyRefMut<'py, Agent>,
        mut creature: PyRefMut<'py, Creature>,
        position: PyRef<'py, Position>,
    ) -> PyResult<Combatant> {
        let creature = creature.creature.take_strong().unwrap();
        let game = PythonOwnedRc::into_inner(self.game.clone());
        let actor = self.game.interface.add_actor(PythonAgent {
            roller: agent.roller.take().unwrap(),
            name: agent.name.clone(),
            coroutine: agent.coroutine.clone_ref(agent.py()),
            game: PythonOwnedRc::downgrade(&self.game),
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

        Ok(Combatant {
            combatant: unsafe { PythonWeak::new(combatant) },
            game: unsafe { PythonWeak::new(PythonOwnedRc::downgrade(&self.game)) },
        })
    }

    pub fn start<'py>(&mut self, py: Python<'py>) -> PyResult<()> {
        py.detach(|| {
            let result = smol::block_on(self.game.combat.start(&self.game).into_future());
            if let Err(err) = result
                && let Ok(err) = err.downcast::<PyErr>()
            {
                return Err(*err);
            }

            Ok(())
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
    pub fn new(name: String, coroutine: Bound<'_, PyAny>, seed: Option<u64>) -> PyResult<Self> {
        let coroutine = Coroutine::new(coroutine)?.unbind();

        let roller = match seed {
            Some(seed) => xander::d20::provider::local_rng::LocalRng::with_seed(seed),
            None => {
                // TODO: Convert this to a more formal Python warning or something...
                eprintln!(
                    "[Xander] You are using a random seed for RNG.\n\tIt is advised to instead use a fixed seed  with `Agent(..., seed=<int>)` for reproducibility."
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
pub struct Combatant {
    pub combatant: PythonWeak<rs::Combatant>,
    pub game: PythonWeak<rs::Game>,
}

impl Combatant {
    pub fn upgrade(&self) -> PyResult<Rc<rs::Combatant>> {
        self.combatant.upgrade_or_expired("Combatant")
    }
}

#[pymethods]
impl Combatant {
    #[getter]
    pub fn name(&self) -> PyResult<String> {
        Ok(self.upgrade()?.creature.name.clone())
    }

    #[getter]
    pub fn current_hp(&self) -> PyResult<u32> {
        Ok(self.upgrade()?.creature.stats.health.current())
    }

    #[getter]
    pub fn max_hp(&self) -> PyResult<u32> {
        let game = self.game.upgrade_or_expired("Game")?;
        let max_hp = run_future(game, self.upgrade()?.creature.stats.health.max_hp.get());
        Ok(max_hp)
    }

    #[pyo3(name = "__repr__")]
    pub fn repr(&self) -> PyResult<String> {
        let name = self.name()?;
        let current_hp = self.current_hp()?;
        let max_hp = self.current_hp()?;

        Ok(format!("{name} <{current_hp}/{max_hp}>"))
    }

    #[getter]
    pub fn creature(&self) -> PyResult<Creature> {
        Ok(Creature {
            creature: MaybeStrong::Weak(Rc::downgrade(&self.upgrade()?.creature)),
            game: self.game.clone(),
        })
    }
}

#[pyclass]
pub struct Creature {
    creature: MaybeStrong<rs::Creature>,
    game: PythonWeak<rs::Game>,
}

#[pymethods]
impl Creature {}

#[pyclass]
pub struct GameEnd(pub rs::GameEndReport);

#[pymethods]
impl GameEnd {
    #[getter]
    pub fn won(&self) -> bool {
        self.0.won
    }
}
