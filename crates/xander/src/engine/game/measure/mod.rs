pub mod currency;
pub mod time;

use std::ops::{Add, Sub};

pub use currency::*;
use rkyv::{Archive, Deserialize, Serialize};

pub const FEET_PER_SQUARE: u32 = 5;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Archive, Serialize, Deserialize,
)]
pub struct Feet<T = u32>(pub T);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Archive, Serialize, Deserialize)]
pub struct Squares(pub u32);

impl Squares {
    pub const fn from_feet_f64(feet: f64) -> Squares {
        Squares((feet / FEET_PER_SQUARE as f64).ceil() as u32)
    }

    pub fn checked_add_signed(&self, rhs: i32) -> Option<Squares> {
        self.0.checked_add_signed(rhs).map(Self)
    }
}

impl From<Feet> for Squares {
    fn from(value: Feet) -> Self {
        Squares(value.0.div_ceil(FEET_PER_SQUARE))
    }
}

impl From<Squares> for Feet {
    fn from(value: Squares) -> Self {
        Feet(value.0 * FEET_PER_SQUARE)
    }
}

/// In seconds.
#[derive(Debug, Clone, Copy, Archive, Serialize, Deserialize)]
pub struct Duration(u32);

const SECS_PER_ROUND: u32 = 6;
const SECS_PER_MIN: u32 = 60;
const MIN_PER_HOUR: u32 = 60;

impl Duration {
    pub const fn rounds(rounds: u32) -> Self {
        Self(rounds * SECS_PER_ROUND)
    }

    #[inline(always)]
    pub const fn round(round: u32) -> Self {
        Self::rounds(round)
    }

    pub const fn hours(hours: u32) -> Self {
        Self(hours * SECS_PER_MIN * MIN_PER_HOUR)
    }

    #[inline(always)]
    pub const fn hour(hour: u32) -> Self {
        Self::hours(hour)
    }

    pub const fn minutes(minutes: u32) -> Self {
        Self(minutes * SECS_PER_MIN)
    }

    #[inline(always)]
    pub const fn minute(minute: u32) -> Self {
        Self::minutes(minute)
    }

    pub const fn seconds(seconds: u32) -> Self {
        Self(seconds)
    }

    #[inline(always)]
    pub const fn second(second: u32) -> Self {
        Self::seconds(second)
    }

    pub const fn as_rounds(self) -> u32 {
        self.0 / SECS_PER_ROUND
    }

    pub const fn as_mins(self) -> u32 {
        self.0 / SECS_PER_MIN
    }

    pub const fn as_hours(self) -> u32 {
        self.0 / SECS_PER_MIN / MIN_PER_HOUR
    }
}

// Math

impl<U, T: Add<U>> Add<Feet<U>> for Feet<T> {
    type Output = Feet<T::Output>;

    fn add(self, rhs: Feet<U>) -> Self::Output {
        Feet(self.0 + rhs.0)
    }
}

impl<U, T: Sub<U>> Sub<Feet<U>> for Feet<T> {
    type Output = Feet<T::Output>;

    fn sub(self, rhs: Feet<U>) -> Self::Output {
        Feet(self.0 - rhs.0)
    }
}
