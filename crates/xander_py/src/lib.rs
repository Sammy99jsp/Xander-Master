#![feature(iter_intersperse)]

pub mod api;
mod py;

use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
mod xander {
    #[pymodule_export]
    use crate::api::game::{Agent, Arena, Combatant, Creature, Game, Position};

    #[pymodule_export]
    use crate::api::turn::{Dash, Disengage, Dodge, Movement, Turn};

    #[pymodule_export]
    use crate::api::reaction::{AttackOfOpportunity, Reaction};

    #[pymodule_export]
    use crate::api::attack::{Attack, AttackReport};

    #[pymodule_export]
    use crate::api::dice::{DExpr, ValTree};

    #[pymodule_export]
    use crate::api::health::{Damage, DamageDice, DamageType};

    #[pymodule_export]
    use crate::api::utils::{Availability, Illegal};
}
