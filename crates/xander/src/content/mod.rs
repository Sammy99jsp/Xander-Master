use dynx::Member;

use crate::engine::game::creature::monster::MonsterType;

macro_rules! monster_types {
    {$($ty: ident: $name: expr),* $(,)?} => {
        $(
            #[derive(Debug)]
            pub struct $ty;

            #[Member($name, register(Singleton))]
            impl MonsterType for $ty {
                fn title(&self) -> &'static str {
                    stringify!($ty)
                }
            }
        )*
    };
}

monster_types! {
    Aberration: "ABERRATION",
    Beast: "BEAST",
    Celestial: "CELESTIAL",
    Construct: "CONSTRUCT",
    Dragon: "DRAGON",
    Elemental: "ELEMENTAL",
    Fey: "FEY",
    Fiend: "FIEND",
    Giant: "GIANT",
    Humanoid: "HUMANOID",
    Monstrosity: "MONSTROSITY",
    Ooze: "OOZE",
    Plant: "PLANT",
    Undead: "UNDEAD",
}
