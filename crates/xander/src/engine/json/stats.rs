use std::collections::HashMap;

use schemars::JsonSchema;

mod rs {
    pub use crate::engine::game::{
        health::{Damage, DamageType},
        stats::Ability,
    };
    pub use d20::DExpr;
}

#[derive(serde::Deserialize, JsonSchema)]
#[serde(try_from = "String")]
#[schemars(example = &"d6")]
pub struct DExpr(#[schemars(with = "&str")] d20::DExpr);

impl TryFrom<String> for DExpr {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        const ERROR: &str = "dice expression parsing error";
        d20::DExpr::try_from(value).map(DExpr).map_err(|_| ERROR)
    }
}

#[derive(serde::Deserialize, JsonSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DamageType {
    Acid,
    Bludgeoning,
    Cold,
    Fire,
    Force,
    Lighting,
    Necrotic,
    Piercing,
    Poison,
    Psychic,
    Radiant,
    Slashing,
    Thunder,
}

impl DamageType {
    pub fn into_rust(self) -> rs::DamageType {
        match self {
            DamageType::Acid => rs::DamageType::Acid,
            DamageType::Bludgeoning => rs::DamageType::Bludgeoning,
            DamageType::Cold => rs::DamageType::Cold,
            DamageType::Fire => rs::DamageType::Fire,
            DamageType::Force => rs::DamageType::Force,
            DamageType::Lighting => rs::DamageType::Lighting,
            DamageType::Necrotic => rs::DamageType::Necrotic,
            DamageType::Piercing => rs::DamageType::Piercing,
            DamageType::Poison => rs::DamageType::Poison,
            DamageType::Psychic => rs::DamageType::Psychic,
            DamageType::Radiant => rs::DamageType::Radiant,
            DamageType::Slashing => rs::DamageType::Slashing,
            DamageType::Thunder => rs::DamageType::Thunder,
        }
    }
}

#[derive(serde::Deserialize, JsonSchema)]
pub struct DamageDice(HashMap<DamageType, DExpr>);

impl From<DamageDice> for rs::Damage<rs::DExpr> {
    fn from(value: DamageDice) -> Self {
        Self::from_iter(value.0.into_iter().map(|(k, v)| (k.into_rust(), v.0)))
    }
}

#[derive(serde::Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Ability {
    #[serde(alias = "str")]
    Strength,
    #[serde(alias = "dex")]
    Dexterity,
    #[serde(alias = "con")]
    Constitution,
    #[serde(alias = "int")]
    Intelligence,
    #[serde(alias = "wis")]
    Wisdom,
    #[serde(alias = "cha")]
    Charisma,
}

impl From<Ability> for rs::Ability {
    fn from(value: Ability) -> Self {
        match value {
            Ability::Strength => rs::Ability::Strength,
            Ability::Dexterity => rs::Ability::Dexterity,
            Ability::Constitution => rs::Ability::Constitution,
            Ability::Intelligence => rs::Ability::Intelligence,
            Ability::Wisdom => rs::Ability::Wisdom,
            Ability::Charisma => rs::Ability::Charisma,
        }
    }
}
