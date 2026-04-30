pub mod character;
pub mod me;
pub mod monster;
pub mod proficiencies;
pub mod size;
pub mod stat_block;

pub use self::{
    character::{Character, Level},
    me::Me,
    monster::{Cr, Monster},
    size::CreatureSize,
    stat_block::StatBlock,
};

use std::rc::Rc;

use xander_runtime::flow::io::Actor;

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug)]
pub struct Creature {
    pub id: CreatureId,
    pub name: String,
    pub size: CreatureSize,
    pub kind: CreatureKind,
    pub stats: StatBlock,
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct CreatureId(u32);

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum CreatureKind {
    Character(character::Character),
    Monster(monster::Monster),
}

impl Creature {
    pub fn new<F>(with_fn: F) -> Rc<Self>
    where
        F: for<'a> FnOnce(Me) -> Self,
    {
        Rc::new_cyclic(move |this| {
            let me = Me(this.clone());
            with_fn(me)
        })
    }

    pub fn actor(&self) -> Actor {
        match &self.kind {
            CreatureKind::Character(character) => character.actor,
            CreatureKind::Monster(_) => Actor::GM,
        }
    }

    pub fn is_dead(&self) -> bool {
        self.stats.health.is_dead()
    }
}

pub mod ui {
    use super::Creature;
    use xander_runtime::{register, ui};

    impl ui::Ui for Creature {}
    register!(Creature, register(Identity("CREATURE")));
}

pub mod provisos {
    use std::future::ready;

    use dynx::Identity;
    use xander_runtime::{
        always_alive,
        lived::provided::{ArchivedProvisoBase, Proviso, ProvisoBase},
        register,
    };

    use super::Me;
    use crate::engine::game::stats::proficiency::ProficiencyBonus;

    #[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct CreatureProficiencyBonus {
        pub me: Me,
    }

    always_alive!(CreatureProficiencyBonus);
    register!(CreatureProficiencyBonus: dyn ProvisoBase<ProficiencyBonus>, register(Archive, Deserialize, Lived));

    impl ArchivedProvisoBase<ProficiencyBonus> for rkyv::Archived<CreatureProficiencyBonus> {}

    impl Identity for CreatureProficiencyBonus {
        type Parent = dyn ProvisoBase<ProficiencyBonus>;
        const LOCAL_ID: &'static str = "CREATURE_PROFICIENCY_BONUS";
    }

    impl Proviso<ProficiencyBonus> for CreatureProficiencyBonus {
        fn provide(
            &self,
            t: &mut ProficiencyBonus,
        ) -> impl IntoFuture<Output = std::ops::ControlFlow<()>> {
            *t = match &self.me.kind {
                super::CreatureKind::Character(_) => todo!(),
                super::CreatureKind::Monster(monster) => monster.cr.proficiency_bonus(),
            };

            ready(std::ops::ControlFlow::Continue(()))
        }
    }
}

#[cfg(test)]
pub fn test_creature() -> Rc<Creature> {
    use self::{
        monster::{Cr, Monster},
        proficiencies::Proficiencies,
        provisos::CreatureProficiencyBonus,
        stat_block::{AbilityModifiers, AbilityScores, base_score as base_score_},
    };
    use crate::engine::game::{
        creature::monster::MonsterType, health::Health, stats::AbilityScore,
    };
    use dynx::{Member, dynx::Single};
    use xander_runtime::lived::Provided;

    fn base_score(s: u8) -> Provided<AbilityScore> {
        base_score_(AbilityScore::try_from(s).unwrap())
    }

    #[derive(Debug)]
    pub struct Test;

    #[Member("TEST", register(Singleton))]
    impl MonsterType for Test {
        fn title(&self) -> &'static str {
            "Test"
        }
    }

    Creature::new(|me| Creature {
        id: CreatureId(0),
        name: "Test-Creature".to_string(),
        size: CreatureSize::Medium,
        kind: CreatureKind::Monster(Monster {
            cr: Cr::Half,
            ty: monster::Type {
                ty: Single::new(&Test),
                tags: Vec::new(),
            },
        }),
        stats: StatBlock {
            me: me.clone(),
            proficiency_bonus: {
                let mut bonus = Provided::new();
                bonus.enroll_mut(CreatureProficiencyBonus { me: me.clone() });
                bonus
            },
            proficiencies: Proficiencies::new(),
            scores: AbilityScores {
                str: base_score(1),
                dex: base_score(6),
                con: base_score(8),
                int: base_score(10),
                wis: base_score(6),
                cha: base_score(12),
            },
            modifiers: AbilityModifiers::new(me.clone()),
            health: Health::with_set_max(me.clone(), 7).unwrap(),
        },
    })
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use dynx::dynx::DynDeserializer;
    use rkyv::{
        Deserialize, access,
        de::Pool,
        rancor::{Error, Strategy},
        to_bytes,
    };

    use crate::engine::game::creature::{Creature, test_creature};

    #[test]
    fn test_serialize_and_deserialize() {
        let creature = test_creature();

        let bytes = to_bytes::<Error>(&creature).unwrap();

        let archived = access::<rkyv::Archived<Rc<Creature>>, Error>(&bytes).unwrap();
        let mut deserializer = Pool::default();
        let deserializer = Strategy::<_, Error>::wrap(&mut deserializer);
        let result: Rc<Creature> = archived
            .get()
            .deserialize(deserializer as &mut dyn DynDeserializer)
            .unwrap();

        println!("{result:?}")
    }
}
