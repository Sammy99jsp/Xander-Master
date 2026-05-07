use std::{fmt::Write, rc::Rc};

use pyo3::{IntoPyObjectExt, prelude::*};

use crate::{
    api::{
        dice::ValTree,
        health::{Damage, DamageDice},
        turn::Combatant,
    },
    py::utils::{OrExpired, PythonWeak, UnsafePythonEscape, run_future},
};

mod rs {
    pub use xander::engine::game::{
        Game,
        combat::{Combatant, attack::*},
        health::DamageReport,
        measure::Feet,
    };
}

#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub struct Attack {
    pub me: PythonWeak<rs::Combatant>,
    pub attack: PythonWeak<rs::Attack>,
    pub game: PythonWeak<rs::Game>,
    pub target: PythonWeak<rs::Combatant>,
}

impl Attack {
    pub fn upgrade(&self) -> PyResult<Rc<rs::Attack>> {
        self.attack.upgrade_or_expired("Combat")
    }
}

#[pymethods]
impl Attack {
    #[getter]
    pub fn name(&self) -> PyResult<String> {
        let attack = self.upgrade()?;
        Ok(attack.name.clone())
    }

    #[getter]
    pub fn range<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let attack = self.upgrade()?;
        match &attack.range() {
            rs::Range::Single(rs::Feet(r)) => r.into_bound_py_any(py),
            rs::Range::Long {
                short: rs::Feet(short),
                long: rs::Feet(long),
            } => (*short, *long).into_bound_py_any(py),
        }
    }

    #[getter]
    #[pyo3(name = "type")]
    pub fn type_(&self) -> PyResult<String> {
        let attack = self.upgrade()?;
        match &attack.kind {
            rs::AttackKind::Melee { .. } => Ok("melee".to_string()),
            rs::AttackKind::Ranged { .. } => Ok("ranged".to_string()),
        }
    }

    #[getter]
    pub fn damage(&self) -> PyResult<DamageDice> {
        let attack = self.upgrade()?;
        let me = self.me.upgrade_or_expired("Combat")?;
        let game = self.game.upgrade_or_expired("Game")?;
        let damage: xander::engine::game::health::Damage<xander::d20::DExpr> =
            run_future(game, attack.damage(&me));

        unsafe { Ok(DamageDice(UnsafePythonEscape::new(damage))) }
    }

    #[getter]
    pub fn target(&self) -> PyResult<Combatant> {
        Ok(Combatant {
            combatant: self.target.clone(),
            game: self.game.clone(),
        })
    }

    #[pyo3(name = "__repr__")]
    pub fn repr(&self) -> PyResult<String> {
        let target = self.target.upgrade_or_expired("Combat")?;
        let attack = self.upgrade()?;
        let me = self.me.upgrade_or_expired("Combat")?;
        let game = self.game.upgrade_or_expired("Combat")?;

        let damage = run_future(game, attack.damage(&me));

        Ok(format!(
            "Attack({}; hit=({}); target={})",
            attack.name, damage, target.creature.name
        ))
    }
}

#[pyclass]
pub struct AttackReport {
    pub report: UnsafePythonEscape<rs::AttackReport>,
}

#[pymethods]
impl AttackReport {
    #[pyo3(name = "__repr__")]
    pub fn repr(&self) -> String {
        let inner = match &self.report.result {
            rs::AttackResult::Miss => {
                format!("Miss: {}", self.report.attack_roll_result)
            }
            rs::AttackResult::Hit { critical, report } => {
                let mut inner = String::new();

                write!(inner, "Hit: {}", self.report.attack_roll_result).unwrap();

                if critical.is_some() {
                    inner.push_str(" (Critical), ")
                } else {
                    inner.push_str(", ");
                }

                match report {
                    Some(rs::DamageReport { dealt, .. }) => {
                        write!(&mut inner, "Damage: {}", dealt.subtotal()).unwrap();
                    }
                    None => inner.push('0'),
                }
                inner
            }
        };

        format!("AttackReport({inner})")
    }

    #[getter]
    pub fn damage(&self) -> Option<Damage> {
        match &self.report.result {
            rs::AttackResult::Hit {
                report: Some(rs::DamageReport { dealt, .. }),
                ..
            } => unsafe { Some(Damage(UnsafePythonEscape::new(dealt.clone()))) },
            _ => None,
        }
    }

    #[getter]
    pub fn hit(&self) -> bool {
        match &self.report.result {
            rs::AttackResult::Hit { .. } => true,
            rs::AttackResult::Miss => false,
        }
    }

    #[getter]
    pub fn to_hit(&self) -> ValTree {
        unsafe {
            ValTree(UnsafePythonEscape::new(
                self.report.attack_roll_result.clone(),
            ))
        }
    }
}
