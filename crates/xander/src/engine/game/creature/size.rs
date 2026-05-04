use crate::engine::game::measure::Squares;

/// Represents the size of a creature.
///
/// Uses [GargantuanDim] to help with discriminant niching
/// to make [CreatureSize], [Option<CreatureSize>] 4 bytes in size.
#[derive(Debug, Clone, Copy)]
pub enum CreatureSize {
    Tiny,
    Small,
    Medium,
    Large,
    Huge,
    Gargantuan(GargantuanDim, GargantuanDim),
}

// Check niches.
const _: () = {
    assert!(size_of::<CreatureSize>() == 4);
    assert!(size_of::<Option<CreatureSize>>() == 4);
};

/// Special niche type representing anything over 20 feet.
///
/// Only values 20 feet and above are allowed.
#[repr(transparent)]
#[derive(Clone, Copy)]
#[rustc_layout_scalar_valid_range_start(20)]
#[rustc_layout_scalar_valid_range_end(0xFFFF)]
pub struct GargantuanDim(u16);

impl std::fmt::Debug for GargantuanDim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl GargantuanDim {
    /// Tries to make a gargantuan size dimension.
    ///
    /// This must be above 20, and a multiple of 5 feet.
    pub fn new(s: u16) -> Option<Self> {
        if !s.is_multiple_of(5) || s < 20 {
            return None;
        }

        unsafe { Some(Self(s)) }
    }
}

impl CreatureSize {
    pub const fn dims_squares(self) -> (Squares, Squares) {
        match self {
            CreatureSize::Tiny => (Squares::from_feet_f64(2.5), Squares::from_feet_f64(2.5)),
            CreatureSize::Small => (Squares::from_feet_f64(5.0), Squares::from_feet_f64(5.0)),
            CreatureSize::Medium => (Squares::from_feet_f64(5.0), Squares::from_feet_f64(5.0)),
            CreatureSize::Large => (Squares::from_feet_f64(10.0), Squares::from_feet_f64(10.0)),
            CreatureSize::Huge => (Squares::from_feet_f64(15.0), Squares::from_feet_f64(15.0)),
            CreatureSize::Gargantuan(GargantuanDim(x), GargantuanDim(y)) => (
                Squares::from_feet_f64(x as f64),
                Squares::from_feet_f64(y as f64),
            ),
        }
    }
}

pub mod archiving {
    use super::{CreatureSize, GargantuanDim};
    use rkyv::{
        Archive, Deserialize, Serialize,
        bytecheck::CheckBytes,
        primitive::ArchivedU16,
        rancor::{Fallible, Source},
        traits::NoUndef,
    };
    use thiserror::Error;

    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, rkyv::Portable)]
    pub struct ArchivedCreatureSize([ArchivedU16; 2]);

    unsafe impl NoUndef for ArchivedCreatureSize {}

    impl ArchivedCreatureSize {
        pub fn from_size(size: CreatureSize) -> Self {
            Self(
                (match size {
                    CreatureSize::Tiny => [1, 0],
                    CreatureSize::Small => [2, 0],
                    CreatureSize::Medium => [3, 0],
                    CreatureSize::Large => [4, 0],
                    CreatureSize::Huge => [5, 0],
                    CreatureSize::Gargantuan(GargantuanDim(x), GargantuanDim(y)) => [x, y],
                })
                .map(ArchivedU16::from_native),
            )
        }

        pub fn as_size(self) -> CreatureSize {
            match self.0.map(ArchivedU16::to_native) {
                [1, 0] => CreatureSize::Tiny,
                [2, 0] => CreatureSize::Small,
                [3, 0] => CreatureSize::Medium,
                [4, 0] => CreatureSize::Large,
                [5, 0] => CreatureSize::Huge,
                [x @ 20..=u16::MAX, y @ 20..=u16::MAX] => unsafe {
                    CreatureSize::Gargantuan(GargantuanDim(x), GargantuanDim(y))
                },
                _ => unreachable!(),
            }
        }
    }

    unsafe impl<C> CheckBytes<C> for ArchivedCreatureSize
    where
        C: Fallible + ?Sized,
        C::Error: Source,
    {
        unsafe fn check_bytes(value: *const Self, _: &mut C) -> Result<(), <C as Fallible>::Error> {
            // Valid pointer: aligned and initialized bytes.
            let value = unsafe {
                value
                    .as_ref()
                    .unwrap_unchecked()
                    .0
                    .map(ArchivedU16::to_native)
            };

            #[derive(Debug, Error)]
            pub enum InvalidCreatureSizeError {
                #[error(
                    "Invalid CreatureSize: expected either (1 = Tiny, 2 = Small, Medium = 3, Large = 4, Huge = 5, Gargantuan = 20+), got {0}."
                )]
                InvalidFirstComponent(u16),

                #[error("For non-gargantuan sizes, the second component must be 0, got: {0}")]
                NonzeroSecondComponent(u16),

                #[error(
                    "Gargantuan sizes must be multiples of 5 ft. and >= 20 ft., got {0} x {1} ft."
                )]
                NotMultipleOfFive(u16, u16),
            }

            match value {
                [1..=5, 0] => Ok(()),
                [1..=5, e] => Err(C::Error::new(
                    InvalidCreatureSizeError::NonzeroSecondComponent(e),
                )),
                [x @ 20..=u16::MAX, y @ 20..=u16::MAX] => {
                    if x.is_multiple_of(5) && y.is_multiple_of(5) {
                        Ok(())
                    } else {
                        Err(C::Error::new(InvalidCreatureSizeError::NotMultipleOfFive(
                            x, y,
                        )))
                    }
                }
                [e, _] => Err(C::Error::new(
                    InvalidCreatureSizeError::InvalidFirstComponent(e),
                )),
            }
        }
    }

    impl Archive for CreatureSize {
        type Archived = ArchivedCreatureSize;
        type Resolver = rkyv::Resolver<u32>;

        fn resolve(&self, _: Self::Resolver, out: rkyv::Place<Self::Archived>) {
            out.write(ArchivedCreatureSize::from_size(*self));
        }
    }

    impl<S> Serialize<S> for CreatureSize
    where
        S: Fallible + ?Sized,
    {
        fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
            Ok(())
        }
    }

    impl<D> Deserialize<CreatureSize, D> for ArchivedCreatureSize
    where
        D: Fallible + ?Sized,
    {
        fn deserialize(&self, _: &mut D) -> Result<CreatureSize, <D as Fallible>::Error> {
            // SAFETY: Same alignment; we have also checked the memory layout.
            Ok(self.as_size())
        }
    }
}
