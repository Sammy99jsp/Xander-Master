use schemars::JsonSchema;
use serde::Deserialize;

use crate::engine::json::creature::Creature;

#[derive(Clone, Deserialize, JsonSchema)]
pub struct Combatant {
    pub creature: Creature,
    pub position: Position,
    pub seed: Seed,
    pub controller: String,
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(untagged, rename_all = "lowercase")]
pub enum Position {
    Random,
    /// Must be in multiples of 5 feet to line up with
    /// the grid squares.
    Fixed(u32, u32),
}

#[derive(Clone, Deserialize, JsonSchema)]
#[serde(untagged, rename_all = "lowercase")]
pub enum Seed {
    Random,
    Fixed(u64),
}
