use rkyv::rancor::{Fallible, Source};

use crate::engine::game::stats::proficiency::ProficiencyBonus;

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Character {}

#[repr(transparent)]
#[rustc_layout_scalar_valid_range_start(1)] // (1)
#[rustc_layout_scalar_valid_range_end(20)] // (20)
#[derive(Debug, Clone, Copy, PartialEq, Eq, rkyv::Archive, rkyv::Serialize)]
pub struct Level(u8);

impl<D> rkyv::Deserialize<Level, D> for rkyv::Archived<Level>
where
    D: Fallible + ?Sized,
    D::Error: Source,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<Level, <D as Fallible>::Error> {
        let inner: u8 = self.0.deserialize(deserializer)?;
        Level::try_from(inner).map_err(D::Error::new)
    }
}

impl Level {
    pub const fn proficiency_bonus(&self) -> ProficiencyBonus {
        // SAFETY: We are within ProficiencyBonus' valid memory layout range of 2..=9
        unsafe {
            match self.0 {
                1..=4 => ProficiencyBonus(2),
                5..=8 => ProficiencyBonus(3),
                9..=12 => ProficiencyBonus(4),
                13..=16 => ProficiencyBonus(5),
                17..=20 => ProficiencyBonus(6),
                0 | 21.. => unreachable!(), // Not valid memory layout for Level
            }
        }

        // TODO: Add unit test for this.
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("{0} is an invalid character level")]
pub struct InvalidLevel(u8);

impl TryFrom<u8> for Level {
    type Error = InvalidLevel;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            v @ (1..=20) => unsafe { Ok(Self(v)) },
            err => Err(InvalidLevel(err)),
        }
    }
    //
}

impl From<Level> for u8 {
    fn from(value: Level) -> Self {
        value.0
    }
}
