pub mod currency;

pub use currency::*;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Archive, Serialize, Deserialize)]
pub struct Feet(pub u32);

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
