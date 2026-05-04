use std::{
    rc::Rc,
    sync::{
        Arc, Weak as ArcWeak,
        atomic::{AtomicBool, Ordering},
    },
};

use numpy::{IntoPyArray, ndarray};
use pyo3::{
    IntoPyObjectExt,
    exceptions::{PyTypeError, PyValueError},
    prelude::*,
};

use crate::{
    api::{
        attack::{Attack, AttackReport},
        utils::{Availability, Illegal},
    },
    py::utils::{OrExpired, PythonWeak, UnsafePythonEscape, run_future},
};

mod rs {
    pub use xander::engine::game::{
        Game,
        combat::{
            Combatant, Timeslot,
            action::{Action, Attacking},
            turn::{self, Turn},
        },
        measure::Feet,
    };
}

#[pyclass]
pub struct Turn {
    pub turn: PythonWeak<rs::Turn>,
    pub end: ArcWeak<AtomicBool>,
    pub game: PythonWeak<rs::Game>,
    pub used: bool,
}

impl Turn {
    fn upgrade(&self) -> PyResult<Rc<rs::Turn>> {
        self.turn.upgrade_or_expired("Turn")
    }

    fn game(&self) -> PyResult<Rc<rs::Game>> {
        self.game.upgrade_or_expired("Game")
    }

    fn mark_used(&mut self) -> PyResult<()> {
        if self.used {
            return Err(PyValueError::new_err(
                "You can only use a Turn object once! Please wait until the next yield!",
            ));
        }

        self.used = true;
        Ok(())
    }
}

#[pymethods]
impl Turn {
    #[getter]
    pub fn movement(&self) -> PyResult<Movement> {
        Ok(Movement {
            turn: self.turn.clone(),
            game: self.game.clone(),
        })
    }

    #[getter]
    pub fn actions<'py>(&self, py: Python<'py>) -> PyResult<Vec<Availability>> {
        let turn = self.upgrade()?;
        let game = self.game()?;
        run_future(game, turn.actions())
            .into_iter()
            .map(|availability| {
                availability
                    .map(|action| to_py_action(py, self.game.clone(), action))
                    .transpose()
                    .map(Availability::from_any)
            })
            .collect::<PyResult<Vec<_>>>()
    }

    pub fn end(&mut self) -> PyResult<()> {
        self.mark_used()?;
        let end: Arc<AtomicBool> = self.end.upgrade_or_expired("Turn")?;
        end.store(true, Ordering::Relaxed);
        Ok(())
    }

    #[pyo3(name = "move")]
    pub fn move_(&mut self, direction: usize) -> PyResult<Option<Illegal>> {
        self.mark_used()?;
        if direction >= rs::turn::DIRECTIONS.len() {
            return Err(PyValueError::new_err(
                "Invalid direction. Directions are indexed 0 to 7 (inclusive), starting from up, top_right, ...",
            ));
        }

        let dir = rs::turn::DIRECTIONS[direction];

        Ok(run_future(self.game()?, self.upgrade()?.move_in(dir))
            .err()
            .map(Illegal::new))
    }

    pub fn take<'py>(
        &mut self,
        py: Python<'py>,
        action: Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        match &action {
            a if let Ok(attack) = a.cast::<Attack>() => {
                Ok(self.attack(py, attack.borrow())?.into_any())
            }
            a if let Ok(_) = a.cast::<Dash>() => Ok(self.dash()?.into_py_any(py)?.into_bound(py)),
            a if let Ok(_) = a.cast::<Disengage>() => {
                Ok(self.disengage()?.into_py_any(py)?.into_bound(py))
            }
            a if let Ok(_) = a.cast::<Dodge>() => Ok(self.dodge()?.into_py_any(py)?.into_bound(py)),
            unexpected => {
                let value = unexpected
                    .repr()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| "Unknown".to_string());
                Err(PyTypeError::new_err(format!(
                    "Expected an Action (any of Attack | Dash | Disengage | Dodge), got {value}",
                )))
            }
        }
    }

    pub fn attack<'py>(
        &mut self,
        py: Python<'py>,
        attack: PyRef<'py, Attack>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let turn = self.upgrade()?;
        let target = attack.target.upgrade_or_expired("Combat")?;
        let attack = attack.upgrade()?;

        let me = turn.combatant.upgrade().unwrap();

        let res = run_future(
            self.game()?,
            attack.attack(&rs::Timeslot::Turn(turn), &me, &target),
        );
        match res {
            Ok(report) => AttackReport {
                report: unsafe { UnsafePythonEscape::new(report) },
            }
            .into_bound_py_any(py),
            Err(illegal) => Illegal::new(illegal).into_bound_py_any(py),
        }
    }

    pub fn dash(&mut self) -> PyResult<Option<Illegal>> {
        self.mark_used()?;
        let turn = self.upgrade()?;
        Ok(run_future(self.game()?, turn.dash())
            .err()
            .map(Illegal::new))
    }

    pub fn disengage(&mut self) -> PyResult<Option<Illegal>> {
        self.mark_used()?;
        let turn = self.upgrade()?;
        Ok(run_future(self.game()?, turn.disengage())
            .err()
            .map(Illegal::new))
    }

    pub fn dodge(&mut self) -> PyResult<Option<Illegal>> {
        self.mark_used()?;
        let turn = self.upgrade()?;
        Ok(run_future(self.game()?, turn.dodge())
            .err()
            .map(Illegal::new))
    }
}

#[pyclass]
pub struct Movement {
    turn: PythonWeak<rs::Turn>,
    game: PythonWeak<rs::Game>,
}

impl Movement {
    fn upgrade(&self) -> PyResult<Rc<rs::Turn>> {
        self.turn.upgrade_or_expired("Turn")
    }

    fn game(&self) -> PyResult<Rc<rs::Game>> {
        self.game.upgrade_or_expired("Game")
    }

    fn _available_directions(&self) -> PyResult<impl Iterator<Item = f32>> {
        Ok(run_future(
            self.game()?,
            self.upgrade()?.available_movement_directions(),
        )
        .into_iter()
        .map(|a| a.is_some() as u8 as f32))
    }
}

#[pymethods]
impl Movement {
    #[getter]
    pub fn speed(&self) -> PyResult<u32> {
        let turn = self.upgrade()?;
        let game: Rc<rs::Game> = self.game()?;

        let me: Rc<rs::Combatant> = turn.combatant.upgrade().unwrap();
        let rs::Feet(speed) = run_future(game, me.creature.stats.speed.get());
        Ok(speed)
    }

    #[getter]
    pub fn used(&self) -> PyResult<u32> {
        Ok(self.upgrade()?.movement.used.get().0)
    }

    #[getter]
    pub fn left(&self) -> PyResult<u32> {
        Ok(run_future(self.game()?, self.upgrade()?.movement_left()).0)
    }

    #[getter]
    pub fn available_directions<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, numpy::PyArray1<f32>>> {
        Ok(ndarray::Array1::from_iter(self._available_directions()?).into_pyarray(py))
    }

    #[pyo3(name = "__repr__")]
    pub fn repr(&self) -> PyResult<String> {
        let display_directions = self
            ._available_directions()?
            .enumerate()
            .map(|(i, a)| {
                if a == 0.0 {
                    rs::turn::DIRECTION_UNAVAILABLE
                } else {
                    rs::turn::DIRECTION_ARROW[i]
                }
            })
            .intersperse(" ")
            .collect::<String>();

        Ok(format!(
            "Movement(speed={speed}, used={used}, left={left}, directions={{ {display_directions} }})",
            speed = self.speed()?,
            used = self.used()?,
            left = self.left()?
        ))
    }
}

macro_rules! action {
    ($id: ident, $name: expr) => {
        #[pyclass]
        pub struct $id;

        #[pymethods]
        impl $id {
            #[pyo3(name = "__repr__")]
            pub fn repr(&self) -> String {
                $name.to_string()
            }

            #[pyo3(name = "__eq__")]
            pub fn eq_(&self, _rhs: &Self) -> bool {
                true
            }
        }
    };
}

action!(Dash, "Dash");
action!(Disengage, "Disengage");
action!(Dodge, "Dodge");

fn to_py_action(
    py: Python<'_>,
    game: PythonWeak<rs::Game>,
    action: rs::Action,
) -> PyResult<Py<PyAny>> {
    match action {
        rs::Action::Dash => Ok(Py::new(py, Dash)?.as_any().clone_ref(py)),
        rs::Action::Disengage => Ok(Py::new(py, Disengage)?.as_any().clone_ref(py)),
        rs::Action::Dodge => Ok(Py::new(py, Dodge)?.as_any().clone_ref(py)),
        rs::Action::Attack(rs::Attacking { me, target, attack }) => Ok(Py::new(
            py,
            Attack {
                me: unsafe { PythonWeak::new(me) },
                target: unsafe { PythonWeak::new(target) },
                attack: unsafe { PythonWeak::new(attack) },
                game,
            },
        )?
        .as_any()
        .clone_ref(py)),
    }
}
