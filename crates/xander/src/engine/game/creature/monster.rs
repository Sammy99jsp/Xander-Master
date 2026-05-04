use std::str::FromStr;

use dynx::{Namespace, dynx::Single};

use crate::engine::game::stats::proficiency::ProficiencyBonus;

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Monster {
    pub cr: Cr,
    pub ty: Type,
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Type {
    pub ty: Single<dyn MonsterType>,
    pub tags: Vec<Single<dyn MonsterTag>>,
}

#[Namespace("MONSTER_TYPE" @ NS, derive(Singleton))]
pub trait MonsterType: std::fmt::Debug {
    fn title(&self) -> &'static str;
}

#[Namespace("MONSTER_TAG" @ TagNS, derive(Singleton))]
pub trait MonsterTag: std::fmt::Debug {
    fn title(&self) -> &'static str;
}

/// # Challenge Ratings
///
/// This is split into an `enum` with a niche-optimized variant for integers,
/// to make this struct fit into one byte, and preserve the u8 memory layout
/// for valid integer CRs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cr {
    /// Use [CR::new] to construct me!
    Integer(IntegerCr),
    Eighth,
    Quarter,
    Half,
}

/// Integer CRs --
#[repr(transparent)]
#[rustc_layout_scalar_valid_range_start(0)] // (0)
#[rustc_layout_scalar_valid_range_end(30)] // (30)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegerCr(u8);

impl std::cmp::PartialOrd for Cr {
    fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
        Some(Ord::cmp(self, rhs))
    }
}

impl std::cmp::Ord for Cr {
    fn cmp(&self, rhs: &Self) -> std::cmp::Ordering {
        // SAFETY: We only return Some(_) from our PartialOrd impl.
        use Cr::*;
        use IntegerCr as Int;
        use std::cmp::Ordering::*;

        // SAFETY: Valid in memory layout range for IntegerCR.
        const ZERO: &Cr = unsafe { &Integer(Int(0)) };

        match (self, rhs) {
            (ZERO, ZERO) => Equal,
            (ZERO, _) => Less,
            (_, ZERO) => Greater,

            (Eighth, Eighth) => Equal,
            (Eighth, _) => Less,
            (_, Eighth) => Greater,

            (Quarter, Quarter) => Equal,
            (Quarter, _) => Less,
            (_, Quarter) => Greater,

            (Half, Half) => Equal,
            (Half, _) => Less,
            (_, Half) => Greater,

            (Integer(Int(lhs @ 1..=30)), Integer(Int(rhs @ 1..=30))) => lhs.cmp(rhs),
            (Integer(_), Integer(_)) => unreachable!("Not valid memory layout for IntegerCR"),
        }

        // TODO: Add unit tests to confirm this order.
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("{0} is out of bounds for a Challenge Rating")]
pub struct OutOfBoundsError(u8);

impl Cr {
    /// Try to construct an integer challenge rating from a u8. Returns [Ok]
    /// if the integer is within range.
    ///
    /// For fractional CRs, construct using their variants directly:
    /// [CR::Eighth], [CR::Quarter], [CR::Half].
    pub const fn try_new(integer: u8) -> Result<Self, OutOfBoundsError> {
        match integer {
            i @ 0..=30 => unsafe { Ok(Self::Integer(IntegerCr(i))) },
            err => Err(OutOfBoundsError(err)),
        }
    }

    /// The [ProficiencyBonus] for this [CR].
    pub const fn proficiency_bonus(&self) -> ProficiencyBonus {
        use Cr::*;
        use IntegerCr as Int;
        match self {
            // SAFETY: These are valid ProficiencyBonus values (2..=9)
            Eighth | Quarter | Half | Integer(Int(..=4)) => unsafe { ProficiencyBonus(2) },
            Integer(Int(5..=8)) => unsafe { ProficiencyBonus(3) },
            Integer(Int(9..=12)) => unsafe { ProficiencyBonus(4) },
            Integer(Int(13..=16)) => unsafe { ProficiencyBonus(5) },
            Integer(Int(17..=20)) => unsafe { ProficiencyBonus(6) },
            Integer(Int(21..=24)) => unsafe { ProficiencyBonus(7) },
            Integer(Int(25..=28)) => unsafe { ProficiencyBonus(8) },
            Integer(Int(29..=30)) => unsafe { ProficiencyBonus(9) },
            Integer(_) => unreachable!(), // Not valid memory layout for IntegerCR
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum CrParseError {
    #[error(transparent)]
    Format(<u8 as FromStr>::Err),
    #[error(transparent)]
    Bounds(OutOfBoundsError),
}

impl TryFrom<String> for Cr {
    type Error = CrParseError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "1/8" => Ok(Self::Eighth),
            "1/4" => Ok(Self::Quarter),
            "1/2" => Ok(Self::Half),
            s => match <u8 as FromStr>::from_str(s) {
                // Valid, in-range integers.
                Ok(i @ 0..=30) => unsafe { Ok(Self::Integer(IntegerCr(i))) },
                // Out-of-bounds integers.
                Ok(err) => Err(CrParseError::Bounds(OutOfBoundsError(err))),
                // Invalid string
                Err(err) => Err(CrParseError::Format(err)),
            },
        }

        // TODO: Add unit test for this.
    }
}

impl std::fmt::Display for Cr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cr::Integer(IntegerCr(i)) => write!(f, "{i}"),
            Cr::Eighth => write!(f, "1/8"),
            Cr::Quarter => write!(f, "1/4"),
            Cr::Half => write!(f, "1/2"),
        }
    }
}

impl From<Cr> for String {
    fn from(value: Cr) -> Self {
        value.to_string()
    }
}

pub mod archiving {
    use rkyv::{
        bytecheck::CheckBytes,
        primitive::ArchivedU32,
        rancor::{Fallible, Source},
    };
    use thiserror::Error;

    use super::{Cr, IntegerCr};

    #[derive(Debug, thiserror::Error)]
    #[error("{0} is not an integer challenge rating")]
    pub struct NonIntegerError(Cr);

    impl TryFrom<Cr> for u8 {
        type Error = NonIntegerError;

        fn try_from(value: Cr) -> Result<Self, Self::Error> {
            match value {
                Cr::Integer(IntegerCr(i)) => Ok(i),
                err => Err(NonIntegerError(err)),
            }
        }
    }

    // Serialization Logic

    /// Archived representation for a CR.
    ///
    /// We essentially use a union between integer CR values
    /// and the fractions as a utf-8 string.
    #[repr(transparent)]
    #[derive(rkyv::Portable, Clone, Copy)]
    pub struct ArchivedCr(ArchivedU32);

    impl ArchivedCr {
        fn from_cr(cr: &Cr) -> Self {
            let bytes = match cr {
                Cr::Integer(IntegerCr(cr)) => *cr as u32,
                Cr::Eighth => u32::from_le_bytes(*b"1/8\0"),
                Cr::Quarter => u32::from_le_bytes(*b"1/4\0"),
                Cr::Half => u32::from_le_bytes(*b"1/2\0"),
            };

            Self(ArchivedU32::from_native(bytes))
        }
    }

    fn check_cr_repr(raw: u32) -> Result<Cr, InvalidArchivedCrError> {
        // Check if it is a simple integer CR.
        if matches!(raw, 0..=30) {
            return unsafe { Ok(Cr::Integer(IntegerCr(raw as u8))) };
        }

        // Otherwise, check to see if it is a fractional CR.
        let as_utf8 = u32::to_le_bytes(raw);
        match &as_utf8 {
            b"1/8\0" => Ok(Cr::Eighth),
            b"1/4\0" => Ok(Cr::Quarter),
            b"1/2\0" => Ok(Cr::Half),

            // Invalid representation.
            _ => Err(InvalidArchivedCrError),
        }
    }

    unsafe impl<C> CheckBytes<C> for ArchivedCr
    where
        C: Fallible + ?Sized,
        C::Error: Source,
    {
        unsafe fn check_bytes(value: *const Self, _: &mut C) -> Result<(), <C as Fallible>::Error> {
            let this = unsafe { *value };
            check_cr_repr(this.0.to_native())
                .map(|_| ())
                .map_err(C::Error::new)
        }
    }

    unsafe impl rkyv::traits::NoUndef for ArchivedCr {}

    impl rkyv::Archive for Cr {
        type Archived = ArchivedCr;
        type Resolver = ();

        fn resolve(&self, (): Self::Resolver, out: rkyv::Place<Self::Archived>) {
            out.write(ArchivedCr::from_cr(self))
        }
    }

    impl<S> rkyv::Serialize<S> for Cr
    where
        S: Fallible + ?Sized,
    {
        fn serialize(
            &self,
            serializer: &mut S,
        ) -> Result<Self::Resolver, <S as rkyv::rancor::Fallible>::Error> {
            let cr = ArchivedCr::from_cr(self);
            cr.0.serialize(serializer)
        }
    }

    #[derive(Debug, Error)]
    #[error("Invalid archived CR, expected either 0-30 or \"1/8\", \"1/4\", \"1/2\" strings.")]
    pub struct InvalidArchivedCrError;

    impl<D> rkyv::Deserialize<Cr, D> for ArchivedCr
    where
        D: Fallible + ?Sized,
        D::Error: Source,
    {
        fn deserialize(&self, _: &mut D) -> Result<Cr, D::Error> {
            check_cr_repr(self.0.to_native()).map_err(D::Error::new)
        }
    }
}
