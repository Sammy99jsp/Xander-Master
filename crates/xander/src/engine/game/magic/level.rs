#[repr(transparent)]
#[rustc_layout_scalar_valid_range_start(0)]
#[rustc_layout_scalar_valid_range_end(9)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Level(pub u8);

impl Level {
    pub const fn new(level: u8) -> Option<Self> {
        match level {
            (0..=9) => unsafe { Some(Self(level)) },
            _ => None,
        }
    }
}
