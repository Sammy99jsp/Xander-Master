use std::{
    rc::Rc,
    sync::{
        Arc, Weak as ArcWeak,
        atomic::{AtomicBool, Ordering},
    },
};

use pyo3::{IntoPyObjectExt, prelude::*};

use crate::{
    api::{
        attack::{Attack, AttackReport},
        game::Me,
        turn::{Combatant, to_py_action},
        utils::{Availability, Illegal},
    },
    py::utils::{OrExpired, PythonWeak, UnsafePythonEscape, run_future},
};

mod rs {
    pub use xander::engine::game::{Game, combat::reaction::AttackOfOpportunity};
}

#[pyclass]
pub struct Reaction {
    pub kind: ReactionKind,
}

#[pymethods]
impl Reaction {
    #[classattr]
    #[pyo3(name = "__match_args__")]
    pub const MATCH_ARGS: (&'static str,) = ("type",);

    #[getter]
    #[pyo3(name = "type")]
    pub fn type_<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        self.kind.clone().into_bound_py_any(py)
    }

    #[getter]
    pub fn me(&self) -> PyResult<Me> {
        match &self.kind {
            ReactionKind::AttackOfOpportunity(aoo) => aoo.me(),
        }
    }

    #[getter]
    pub fn actions<'py>(&self, py: Python<'py>) -> PyResult<Vec<Availability>> {
        match &self.kind {
            ReactionKind::AttackOfOpportunity(aoo) => aoo.actions(py),
        }
    }

    pub fn take<'py>(
        &self,
        py: Python<'py>,
        attack: PyRef<'py, Attack>,
    ) -> PyResult<Bound<'py, PyAny>> {
        match &self.kind {
            ReactionKind::AttackOfOpportunity(aoo) => aoo.take(py, attack)
        }
    }

}
#[doc(hidden)]
#[derive(Clone, IntoPyObject)]
pub enum ReactionKind {
    AttackOfOpportunity(AttackOfOpportunity),
}

#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct AttackOfOpportunity {
    pub aoo: PythonWeak<rs::AttackOfOpportunity>,
    pub end: ArcWeak<AtomicBool>,
    pub game: PythonWeak<rs::Game>,
}
impl AttackOfOpportunity {
    fn upgrade(&self) -> PyResult<Rc<rs::AttackOfOpportunity>> {
        self.aoo.upgrade_or_expired("Attack of opportunity")
    }

    fn set_end(&self) -> PyResult<()> {
        let end: Arc<AtomicBool> = self.end.upgrade_or_expired("Attack of opportunity")?;

        end.store(true, Ordering::Relaxed);

        Ok(())
    }
}

#[pymethods]
impl AttackOfOpportunity {
    #[getter]
    pub fn actions(&self, py: Python<'_>) -> PyResult<Vec<Availability>> {
        let aoo = self.upgrade()?;
        let game = self.game.upgrade_or_expired("Game")?;
        run_future(game, aoo.actions())
            .into_iter()
            .map(|availability| {
                availability
                    .map(|a| to_py_action(py, self.game.clone(), a))
                    .transpose()
            })
            .map(|availability| availability.map(Availability::from_any))
            .collect::<PyResult<Vec<_>>>()
    }

    #[getter]
    pub fn target(&self) -> PyResult<Combatant> {
        let aoo = self.upgrade()?;
        Ok(Combatant {
            combatant: unsafe { PythonWeak::new(aoo.target.clone()) },
            game: self.game.clone(),
        })
    }

    pub fn take<'py>(
        &self,
        py: Python<'py>,
        attack: PyRef<'py, Attack>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.set_end()?;
        let game = self.game.upgrade_or_expired("Game")?;
        let aoo = self.upgrade()?;
        let attack = attack.upgrade()?;

        let res = run_future(game, aoo.attack(&attack));
        match res {
            Ok(report) => AttackReport {
                report: unsafe { UnsafePythonEscape::new(report) },
            }
            .into_bound_py_any(py),
            Err(illegal) => Illegal::new(illegal).into_bound_py_any(py),
        }
    }

    pub fn skip(&self) -> PyResult<()> {
        self.set_end()?;
        let end: Arc<AtomicBool> = self.end.upgrade().unwrap();
        end.store(true, Ordering::Relaxed);
        Ok(())
    }

    #[getter]
    pub fn me(&self) -> PyResult<Me> {
        Ok(Me {
            me: unsafe { PythonWeak::new(self.upgrade()?.me.clone()) },
            game: self.game.clone(),
        })
    }
}
