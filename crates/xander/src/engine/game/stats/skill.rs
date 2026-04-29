use xander_runtime::{identity, register};

use crate::engine::game::stats::proficiency::{
    ArchivedProficiencyApplicationBase, ProficiencyApplication, ProficiencyApplicationBase,
};

use super::ability::prelude::*;

macro_rules! skills {
    ($s_skill: ident: $s_ability: expr, $($skills: ident: $abilities: expr),*$(,)?) => {
        #[repr(u8)]
        #[derive(Debug, Clone, Copy,  PartialEq, Eq, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
        pub enum Skill {
            $s_skill = 7,
            $($skills),*
        }

        impl Skill {
            pub const fn len() -> usize {
                #[allow(non_snake_case)]
                const {
                    let $s_skill: usize = 1;
                    $(let $skills: usize = 1;)*
                    $s_skill $(+ $skills)*
                }
            }

            #[inline(always)]
            pub const fn as_index(self) -> usize {
                self as u8 as usize - 7
            }

            #[inline]
            pub const fn ability(self) -> Ability {
                const BASE_ABILITIES: &[Ability] = &[
                    $s_ability, $($abilities),*,
                ];

                BASE_ABILITIES[self.as_index()]
            }
        }
    };
}

skills!(
    Acrobatics: Dexterity,
    AnimalHandling: Wisdom,
    Arcana: Intelligence,
    Athletics: Strength,
    Deception: Charisma,
    History: Intelligence,
    Insight: Wisdom,
    Intimidation: Charisma,
    Investigation: Intelligence,
    Medicine: Wisdom,
    Nature: Intelligence,
    Perception: Wisdom,
    Performance: Charisma,
    Persuasion: Charisma,
    Religion: Intelligence,
    SleightOfHand: Dexterity,
    Stealth: Dexterity,
    Survival: Wisdom,
);

impl ProficiencyApplication for Skill {}
impl ArchivedProficiencyApplicationBase for ArchivedSkill {}

identity!(Skill: dyn ProficiencyApplicationBase, "SKILL");
register!(Skill: dyn ProficiencyApplicationBase, register(Archive, Deserialize));

pub mod profs {
    use xander_runtime::always_alive;

    use crate::engine::game::stats::proficiency::{
        ArchivedProficiencyBase, Proficiency, ProficiencyBase,
    };

    use super::*;

    #[derive(Debug, Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct SkillProficiency {
        pub skill: Skill,
    }

    always_alive!(SkillProficiency);

    identity!(SkillProficiency: dyn ProficiencyBase, "SKILL");
    register!(SkillProficiency: dyn ProficiencyBase, register(Archive, Deserialize, Lived));

    impl ArchivedProficiencyBase for ArchivedSkillProficiency {}

    impl Proficiency for SkillProficiency {
        type Application = Skill;

        fn applies_to(&self, app: &Self::Application) -> bool {
            self.skill == *app
        }
    }
}
