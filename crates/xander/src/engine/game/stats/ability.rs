use std::{
    ops::{Add, Sub},
    rc::{Rc, Weak},
};

use crate::engine::game::creature::Creature;
use d20::{BinaryOperator as BinOp, DExpr};
use thiserror::Error;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum Ability {
    Strength = 0,
    Dexterity = 1,
    Constitution = 2,
    Intelligence = 3,
    Wisdom = 4,
    Charisma = 5,
}

impl Ability {
    pub const fn len() -> usize {
        6
    }

    pub const fn as_index(self) -> usize {
        self as u8 as usize
    }
}

pub mod prelude {
    pub use super::Ability::*;
    pub use super::{Ability, AbilityModifier, AbilityScore};
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, rkyv::Archive, rkyv::Serialize)]
#[rustc_layout_scalar_valid_range_start(1)]
#[rustc_layout_scalar_valid_range_end(30)]
pub struct AbilityScore(u8);

impl Default for AbilityScore {
    fn default() -> Self {
        unsafe { Self(0) }
    }
}

#[derive(Debug, Clone, Copy, Error)]
#[error("{0} is not a valid ability score (between 1 and 30, inclusive)")]
pub struct IllegalAbilityScore(u8);
impl TryFrom<u8> for AbilityScore {
    type Error = IllegalAbilityScore;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1..=30 => unsafe { Ok(Self(value)) },
            erroneous => Err(IllegalAbilityScore(erroneous)),
        }
    }
}

impl From<AbilityScore> for u8 {
    fn from(score: AbilityScore) -> Self {
        score.0
    }
}

impl AbilityScore {
    /// Special ability score of 0.
    pub const ZERO: Self = unsafe { Self(0) };

    pub const fn modifier(self) -> AbilityModifier {
        // We're alright to cast to i8 since (0..=30) is within (-128..=128).
        // Sub 10 and Half (rounding down to negative infinity).

        // This is slightly annoying because #[feature(int_roundings)] is not stable yet...
        let val = (self.0 as i8 - 10i8).div_floor(2);

        // SAFETY:  For all x in [0, +30], there exists y in [-5, +10],
        //          where y = floor((x - 10) / 2).
        //          We also verify this with some unit tests.
        unsafe { AbilityModifier(val) }
    }

    pub const fn value(self) -> u8 {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
#[rustc_layout_scalar_valid_range_start(0b1111_1011)] // -5
#[rustc_layout_scalar_valid_range_end(10)] // +10
pub struct AbilityModifier(i8);

impl AbilityModifier {
    pub fn into_expr(&self, ability: Ability, me: Weak<Creature>) -> d20::DExpr {
        d20::DExpr::from(self.0 as i32).label(Rc::new(ui::CreatureAbilityModifier { me, ability }))
    }
}

impl From<AbilityModifier> for i32 {
    fn from(value: AbilityModifier) -> Self {
        value.0 as i32
    }
}

impl Default for AbilityModifier {
    fn default() -> Self {
        unsafe { Self(0) }
    }
}

// Useful math for ability modifiers

impl Add<AbilityModifier> for DExpr {
    type Output = DExpr;

    fn add(self, rhs: AbilityModifier) -> Self::Output {
        DExpr::BinaryOperation(Box::new(self), BinOp::Add, Box::new(i32::from(rhs).into()))
    }
}

impl Add<DExpr> for AbilityModifier {
    type Output = DExpr;

    fn add(self, rhs: DExpr) -> Self::Output {
        DExpr::BinaryOperation(Box::new(i32::from(self).into()), BinOp::Add, Box::new(rhs))
    }
}

impl Sub<AbilityModifier> for DExpr {
    type Output = DExpr;

    fn sub(self, rhs: AbilityModifier) -> Self::Output {
        DExpr::BinaryOperation(Box::new(self), BinOp::Sub, Box::new(i32::from(rhs).into()))
    }
}

impl Sub<DExpr> for AbilityModifier {
    type Output = DExpr;

    fn sub(self, rhs: DExpr) -> Self::Output {
        DExpr::BinaryOperation(Box::new(i32::from(self).into()), BinOp::Sub, Box::new(rhs))
    }
}

// Proficiency for saving throws

pub mod profs {
    use crate::prelude::proficiency::*;

    register!(super::Ability: dyn ProficiencyApplicationBase, register(Identity("ABILITY"), Archive, Deserialize));

    impl ArchivedProficiencyApplicationBase for rkyv::Archived<super::Ability> {}
    impl ProficiencyApplication for super::Ability {}
}

pub mod archiving {
    use super::*;
    use rkyv::{
        Deserialize,
        rancor::{Fallible, Source},
    };

    impl<D> Deserialize<AbilityScore, D> for rkyv::Archived<AbilityScore>
    where
        D: Fallible + ?Sized,
        D::Error: Source,
    {
        fn deserialize(&self, deserializer: &mut D) -> Result<AbilityScore, D::Error> {
            let inner = self.0.deserialize(deserializer)?;
            AbilityScore::try_from(inner).map_err(D::Error::new)
        }
    }
}

pub mod ui {
    use std::rc::Weak;

    use xander_runtime::{register, ui};

    use crate::engine::game::{creature::Creature, stats::Ability};

    #[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct CreatureAbilityModifier {
        pub me: Weak<Creature>,
        pub ability: Ability,
    }

    impl ui::Ui for CreatureAbilityModifier {}
    register!(
        CreatureAbilityModifier,
        register(Identity("CREATURE_ABILITY_MODIFIER"))
    );
}
