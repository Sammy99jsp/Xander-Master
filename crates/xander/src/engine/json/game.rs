use schemars::JsonSchema;
use serde::Deserialize;

use crate::engine::json::combatant::Combatant;

#[derive(Deserialize, JsonSchema)]
pub struct Game {
    #[schemars(description = "Dimensions of the arena. Must be a multiple of 5.", example = [30, 30], example = [120, 120])]
    pub arena: (u32, u32),
    pub combatants: Vec<Combatant>,
}
