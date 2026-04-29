//! ## Schools of Magic

use const_format::Case;
use dynx::{Member, Namespace};
use xander_runtime::ui;

#[Namespace("SPELL::SCHOOL" @ NS, derive(Singleton))]
pub trait School: ui::Ui {}

macro_rules! schools_of_magic {
    {
        $(
            $(#[$($tt: tt)*])*
            pub struct $school: ident;
        )*
    } => {
        $(
            $(#[$($tt)*])*
            pub struct $school;

            #[Member(const_format::map_ascii_case!(Case::UpperSnake, stringify!($school)), register(Singleton))]
            impl School for $school {}

            impl ui::Ui for $school {}
        )*
    };
}

schools_of_magic! {
    #[derive(Debug, Clone, Copy)]
    pub struct Abjuration;

    #[derive(Debug, Clone, Copy)]
    pub struct Conjuration;

    #[derive(Debug, Clone, Copy)]
    pub struct Divination;

    #[derive(Debug, Clone, Copy)]
    pub struct Enchantment;

    #[derive(Debug, Clone, Copy)]
    pub struct Evocation;

    #[derive(Debug, Clone, Copy)]
    pub struct Illusion;

    #[derive(Debug, Clone, Copy)]
    pub struct Necromancy;

    #[derive(Debug, Clone, Copy)]
    pub struct Transmutation;
}

#[cfg(test)]
mod tests {
    use xander_runtime::dynx::Identity;

    use crate::engine::game::magic::school::Necromancy;

    #[test]
    fn test_name_case() {
        assert_eq!(Necromancy::LOCAL_ID, "NECROMANCY")
    }
}
