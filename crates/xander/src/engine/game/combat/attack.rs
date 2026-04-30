use dynx::{Namespace, dynx::Single};

use crate::engine::game::measure::Feet;

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Attack {
    pub name: String,
    pub base: Single<dyn AttackBase>,
    pub meta: AttackMeta,
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum AttackMeta {
    Melee {
        reach: Option<Feet>,
        roll: i32,
        hit: d20::DExpr,
    },
}

#[Namespace("ATTACK" @ NS, derive(Singleton))]
pub trait AttackBase: std::fmt::Debug {}
