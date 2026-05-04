use std::ops::{Add, Sub};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct Rounds(pub u32);

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct Turns(pub u32);

macro_rules! ops {
    ($p: path => [$($tr: ident :: $f: ident),* $(,)?]) => {
        $(
            impl $tr for $p {
                type Output = Self;

                fn $f(self, rhs: Self) -> Self::Output {
                    Self($tr::$f(self.0, rhs.0))
                }
            }
        )*
    };
}

ops!(Rounds => [Add::add, Sub::sub]);
ops!(Turns => [Add::add, Sub::sub]);
