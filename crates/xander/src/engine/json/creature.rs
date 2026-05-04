use std::{num::NonZeroU32, rc::Rc};

use schemars::JsonSchema;
use serde::Deserialize;
use thiserror::Error;

mod rs {
    pub use crate::{
        d20::DExpr,
        engine::game::{
            combat::action::{
                Attack,
                attack::{AttackBase, AttackKind, Range, SetMonsterAttack},
            },
            creature::{
                Cr, Creature, CreatureId, CreatureKind, CreatureSize,
                actions::{Actions, Attacks},
                monster::{Monster, MonsterTag, MonsterType, Type},
                provisos::SetSpeed,
                size::GargantuanDim,
                stat_block::{self, AbilityScores, StatBlock},
            },
            health::{
                Health,
                riv::{
                    DamageEffect, DamageFilter, Immunity, ImmunityTarget, Resistance, Vulnerability,
                },
            },
            measure::Feet,
            stats::{AbilityScore, d20_test::attack_roll::provisos::SetAc},
        },
        runtime::{
            dynx::dynx::Single,
            lived::{OptionalDependency, Provided},
        },
    };
}

use crate::engine::json::{
    stats::{Ability, DamageDice, DamageType},
    utils::{Single, WithId},
};

#[derive(Deserialize, JsonSchema)]
pub struct Creature {
    pub name: String,
    #[schemars(with = "CreatureSizeInner")]
    pub size: CreatureSize,
    pub kind: CreatureKind,
    pub stats: Stats,
}

#[derive(Deserialize)]
#[serde(try_from = "CreatureSizeInner")]
pub struct CreatureSize(pub rs::CreatureSize);

#[derive(Deserialize, JsonSchema)]
pub struct GargantuanDimension(#[schemars(range(min = 20,))] pub u16);

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CreatureSizeInner {
    Tiny,
    Small,
    Medium,
    Large,
    Gargantuan(Option<(GargantuanDimension, GargantuanDimension)>),
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CreatureKind {
    Monster(Monster),
}

#[doc(hidden)]
#[derive(JsonSchema)]
#[schemars(untagged)]
pub enum CrRaw {
    String(String),
    U8(u8),
}

#[derive(Deserialize, JsonSchema)]
pub struct Monster {
    #[serde(deserialize_with = "visitors::deserialize_cr")]
    #[schemars(with = "CrRaw", example = 2, example=&"1/4")]
    pub cr: rs::Cr,
    #[serde(rename = "type")]
    pub ty: MonsterType,
}

#[derive(Deserialize, JsonSchema)]
pub struct MonsterType {
    #[serde(rename = "type")]
    #[schemars(with = "String")]
    pub ty: Single<dyn rs::MonsterType>,

    #[serde(default)]
    #[schemars(with = "Vec<String>")]
    pub tags: Vec<Single<dyn rs::MonsterTag>>,
}

#[derive(Deserialize, JsonSchema)]
pub struct Stats {
    pub speed: u32,
    // pub proficiencies: Vec<Proficiency>,
    pub scores: AbilityScores,
    pub health: Health,
    pub actions: Actions,
    pub ac: i32,
}

#[derive(Deserialize, JsonSchema)]
pub struct AbilityScores {
    pub str: AbilityScore,
    pub dex: AbilityScore,
    pub con: AbilityScore,
    pub int: AbilityScore,
    pub wis: AbilityScore,
    pub cha: AbilityScore,
}

#[derive(Deserialize, JsonSchema)]
#[serde(try_from = "u8")]
pub struct AbilityScore(#[schemars(with = "u8", range(min = 1, max = 30))] rs::AbilityScore);

impl TryFrom<u8> for AbilityScore {
    type Error = <rs::AbilityScore as TryFrom<u8>>::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        rs::AbilityScore::try_from(value).map(Self)
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct Health {
    pub max_hp: NonZeroU32,

    #[serde(default)]
    pub resistances: Vec<DamageType>,

    #[serde(default)]
    pub immunities: Vec<DamageType>,

    #[serde(default)]
    pub vulnerabilities: Vec<DamageType>,
}

#[derive(Deserialize, JsonSchema)]
pub struct Actions {
    pub attacks: Vec<Attack>,
}

#[derive(Deserialize, JsonSchema)]
pub struct Attack {
    pub name: String,
    pub kind: AttackKind,
    pub hit: DamageDice,
    // pub prof: ...
    pub ability: Ability,
}

#[derive(Deserialize, JsonSchema)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum AttackKind {
    Melee {
        #[serde(default)]
        reach: Option<u32>,
    },
    Ranged {
        #[serde(flatten)]
        range: Range,
    },
}

#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Range {
    Single(u32),
    Long([u32; 2]),
}

mod visitors {
    use serde::{Deserializer, de::Unexpected};

    use crate::engine::game::creature::Cr;

    pub fn deserialize_cr<'de, D>(deserializer: D) -> Result<Cr, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Cr;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    f,
                    "a monster challenge rating (either '1/8' | '1/4' | '1/2', or integer 0..=30)"
                )
            }

            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Cr::try_new(v).map_err(|_| E::invalid_value(Unexpected::Unsigned(v as u64), &self))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Cr::try_from(v).map_err(|_| E::invalid_value(Unexpected::Str(v), &self))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl From<WithId<Creature, rs::CreatureId>> for Rc<rs::Creature> {
    fn from(value: WithId<Creature, rs::CreatureId>) -> Self {
        let WithId {
            value:
                Creature {
                    name,
                    size,
                    kind,
                    stats,
                },
            id,
        } = value;

        rs::Creature::new(|me| rs::Creature {
            id,
            name,
            size: size.0,
            kind: match kind {
                CreatureKind::Monster(Monster {
                    cr,
                    ty: MonsterType { ty, tags },
                }) => rs::CreatureKind::Monster(rs::Monster {
                    cr,
                    ty: rs::Type {
                        ty: ty.0,
                        tags: tags.into_iter().map(|Single(s)| s).collect(),
                    },
                }),
            },
            stats: {
                let Stats {
                    speed,
                    scores,
                    health,
                    actions,
                    ac,
                } = stats;
                rs::StatBlock {
                    me: me.clone(),
                    proficiency_bonus: rs::stat_block::defaults::proficiency_bonus(me.clone()),
                    speed: {
                        let mut provided = rs::Provided::new();
                        provided.enroll_mut(rs::SetSpeed { speed });
                        provided
                    },
                    proficiencies: rs::stat_block::defaults::proficiencies(),
                    scores: rs::AbilityScores {
                        str: rs::stat_block::base_score(scores.str.0),
                        dex: rs::stat_block::base_score(scores.dex.0),
                        con: rs::stat_block::base_score(scores.con.0),
                        int: rs::stat_block::base_score(scores.int.0),
                        wis: rs::stat_block::base_score(scores.wis.0),
                        cha: rs::stat_block::base_score(scores.cha.0),
                    },
                    modifiers: rs::stat_block::defaults::modifiers(me.clone()),
                    health: {
                        let mut h =
                            rs::Health::with_set_max(me.clone(), health.max_hp.get()).unwrap();

                        fn damage_effect(to: rs::DamageFilter) -> rs::DamageEffect {
                            rs::DamageEffect {
                                dep: rs::OptionalDependency::new(None),
                                to,
                            }
                        }

                        h.riv.resistances.get_mut().extend(
                            health
                                .resistances
                                .into_iter()
                                .map(DamageType::into_rust)
                                .map(rs::DamageFilter::Type)
                                .map(damage_effect)
                                .map(rs::Resistance)
                                .map(Rc::new),
                        );
                        h.riv.immunities.get_mut().extend(
                            health
                                .immunities
                                .into_iter()
                                .map(DamageType::into_rust)
                                .map(rs::DamageFilter::Type)
                                .map(rs::ImmunityTarget::Damage)
                                .map(|to| rs::Immunity {
                                    dep: rs::OptionalDependency::new(None),
                                    to,
                                })
                                .map(Rc::new),
                        );
                        h.riv.vulnerabilities.get_mut().extend(
                            health
                                .vulnerabilities
                                .into_iter()
                                .map(DamageType::into_rust)
                                .map(rs::DamageFilter::Type)
                                .map(damage_effect)
                                .map(rs::Vulnerability)
                                .map(Rc::new),
                        );
                        h
                    },
                    actions: {
                        rs::Actions {
                            attacks: {
                                let mut attacks = rs::Attacks::new(me.clone());
                                attacks
                                    .attacks
                                    .get_mut()
                                    .extend(actions.attacks.into_iter().map(
                                        |Attack {
                                             name,
                                             kind,
                                             hit,
                                             ability,
                                         }| {
                                            Rc::new(rs::Attack {
                                                name,
                                                base: rs::Single::new(
                                                    &rs::SetMonsterAttack as &dyn rs::AttackBase,
                                                ),
                                                kind: kind.into(),
                                                hit: hit.into(),
                                                prof: None,
                                                ability: ability.into(),
                                            })
                                        },
                                    ));
                                attacks
                            },
                        }
                    },
                    reaction: rs::stat_block::defaults::reaction(me.clone()),
                    ac: {
                        let mut provided = rs::Provided::new();
                        provided.enroll_mut(rs::SetAc(rs::DExpr::from(ac)));
                        provided
                    },
                    markers: rs::stat_block::defaults::markers(),
                }
            },
        })
    }
}

#[derive(Debug, Error)]
#[error("This creature's size ({0} x {1} ft.) is too low to be Gargantuan")]
pub struct InvalidCreatureSize(u16, u16);

impl TryFrom<CreatureSizeInner> for CreatureSize {
    type Error = InvalidCreatureSize;

    fn try_from(value: CreatureSizeInner) -> Result<Self, Self::Error> {
        Ok(Self(match value {
            CreatureSizeInner::Tiny => rs::CreatureSize::Tiny,
            CreatureSizeInner::Small => rs::CreatureSize::Small,
            CreatureSizeInner::Medium => rs::CreatureSize::Huge,
            CreatureSizeInner::Large => rs::CreatureSize::Large,
            CreatureSizeInner::Gargantuan(None) => rs::CreatureSize::Gargantuan(
                rs::GargantuanDim::new(20).unwrap(),
                rs::GargantuanDim::new(20).unwrap(),
            ),
            CreatureSizeInner::Gargantuan(Some((
                GargantuanDimension(w),
                GargantuanDimension(h),
            ))) => {
                let r1 = rs::GargantuanDim::new(w).ok_or(w);
                let r2 = rs::GargantuanDim::new(h).ok_or(h);
                match r1.and_then(move |w| r2.map(|h| (w, h))) {
                    Ok((w, h)) => rs::CreatureSize::Gargantuan(w, h),
                    _ => return Err(InvalidCreatureSize(w, h)),
                }
            }
        }))
    }
}

impl From<AttackKind> for rs::AttackKind {
    fn from(value: AttackKind) -> Self {
        match value {
            AttackKind::Melee { reach } => rs::AttackKind::Melee {
                reach: reach.map(rs::Feet),
            },
            AttackKind::Ranged {
                range: Range::Single(single),
            } => rs::AttackKind::Ranged {
                range: rs::Range::Single(rs::Feet(single)),
            },
            AttackKind::Ranged {
                range: Range::Long([short, long]),
            } => rs::AttackKind::Ranged {
                range: rs::Range::Long {
                    short: rs::Feet(short),
                    long: rs::Feet(long),
                },
            },
        }
    }
}
