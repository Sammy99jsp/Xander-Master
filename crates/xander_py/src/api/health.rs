use pyo3::prelude::*;

use crate::{api::dice::{DExpr, ValTree}, py::utils::UnsafePythonEscape};

mod rs {
    pub use xander::{
        d20,
        engine::game::health::{Damage, DamageType},
    };
}

#[pyclass]
pub struct DamageType(rs::DamageType);

#[pymethods]
impl DamageType {
    #[pyo3(name = "__repr__")]
    pub fn repr(&self) -> String {
        format!("{}", self.0)
    }

    #[pyo3(name = "__eq__")]
    pub fn eq_(&self, other: PyRef<'_, Self>) -> bool {
        self.0 == other.0
    }

    #[classattr]
    #[pyo3(name = "Acid")]
    const ACID: Self = Self(rs::DamageType::Acid);
    #[classattr]
    #[pyo3(name = "Bludgeoning")]
    const BLUDGEONING: Self = Self(rs::DamageType::Bludgeoning);
    #[classattr]
    #[pyo3(name = "Cold")]
    const COLD: Self = Self(rs::DamageType::Cold);
    #[classattr]
    #[pyo3(name = "Fire")]
    const FIRE: Self = Self(rs::DamageType::Fire);
    #[classattr]
    #[pyo3(name = "Force")]
    const FORCE: Self = Self(rs::DamageType::Force);
    #[classattr]
    #[pyo3(name = "Lighting")]
    const LIGHTING: Self = Self(rs::DamageType::Lighting);
    #[classattr]
    #[pyo3(name = "Necrotic")]
    const NECROTIC: Self = Self(rs::DamageType::Necrotic);
    #[classattr]
    #[pyo3(name = "Piercing")]
    const PIERCING: Self = Self(rs::DamageType::Piercing);
    #[classattr]
    #[pyo3(name = "Poison")]
    const POISON: Self = Self(rs::DamageType::Poison);
    #[classattr]
    #[pyo3(name = "Psychic")]
    const PSYCHIC: Self = Self(rs::DamageType::Psychic);
    #[classattr]
    #[pyo3(name = "Radiant")]
    const RADIANT: Self = Self(rs::DamageType::Radiant);
    #[classattr]
    #[pyo3(name = "Slashing")]
    const SLASHING: Self = Self(rs::DamageType::Slashing);
    #[classattr]
    #[pyo3(name = "Thunder")]
    const THUNDER: Self = Self(rs::DamageType::Thunder);
}

#[pyclass]
pub struct DamageDice(pub UnsafePythonEscape<rs::Damage<rs::d20::DExpr>>);

#[pymethods]
impl DamageDice {
    #[pyo3(name = "__repr__")]
    pub fn repr(&self) -> String {
        self.0.to_string()
    }

    pub fn sum(&self) -> DExpr {
        unsafe { DExpr(UnsafePythonEscape::new(self.0.sum())) }
    }
}

#[pyclass]
pub struct Damage(pub UnsafePythonEscape<rs::Damage<rs::d20::ValTree>>);

#[pymethods]
impl Damage {
    #[pyo3(name = "__repr__")]
    pub fn repr(&self) -> String {
        self.0.to_string()
    }

    pub fn sum(&self) -> ValTree {
        unsafe { ValTree(UnsafePythonEscape::new(self.0.sum())) }
    }

    pub fn total(&self) -> i32 {
        self.0.total()
    }

    #[pyo3(name = "__int__")]
    pub fn to_int(&self) -> i32 {
        self.total()
    }
}
