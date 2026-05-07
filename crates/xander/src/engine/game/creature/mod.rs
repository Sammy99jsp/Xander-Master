pub mod actions;
pub mod character;
pub mod marker;
pub mod me;
pub mod monster;
pub mod proficiencies;
pub mod size;
pub mod stat_block;

use crate::engine::game::{
    combat::{Combatant, affiliation::Affiliation, arena::Position, attack::test_attack},
    measure::Squares,
    stats::d20_test::attack_roll::provisos::SetAc,
};

pub use self::{
    character::{Character, Level},
    me::Me,
    monster::{Cr, Monster},
    size::CreatureSize,
    stat_block::StatBlock,
};

use std::cell::Cell;
use std::rc::Rc;

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug)]
pub struct Creature {
    pub id: CreatureId,
    pub name: String,
    pub size: CreatureSize,
    pub kind: CreatureKind,
    pub stats: StatBlock,
}

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct CreatureId(pub u32);

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

    pub fn is_dead(&self) -> bool {
        self.stats.health.is_dead()
    }

    pub fn can_take_turns(&self) -> bool {
        !self.is_dead()
    }

    pub fn me(self: &Rc<Self>) -> Me {
        Me(Rc::downgrade(self))
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

    use xander_runtime::{
        lived::provided::{ArchivedProvisoBase, Proviso, ProvisoBase},
        register,
    };

    use super::Me;
    use crate::engine::game::{measure::Feet, stats::proficiency::ProficiencyBonus};

    #[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct CreatureProficiencyBonus {
        pub me: Me,
    }

    register!(CreatureProficiencyBonus: dyn ProvisoBase<ProficiencyBonus>, register(Archive, Deserialize, Lived(always), Identity("CREATURE::PROFICIENCY_BONUS")));

    impl ArchivedProvisoBase<ProficiencyBonus> for rkyv::Archived<CreatureProficiencyBonus> {}

    // impl Identity for CreatureProficiencyBonus {
    //     type Parent = dyn ProvisoBase<ProficiencyBonus>;
    //     const LOCAL_ID: &'static str = "CREATURE_PROFICIENCY_BONUS";
    // }

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

    #[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct SetSpeed {
        pub speed: u32,
    }

    impl ArchivedProvisoBase<Feet> for rkyv::Archived<SetSpeed> {}
    register!(SetSpeed: dyn ProvisoBase<Feet>, register(Archive, Deserialize, Lived(always), Identity("CREATURE::SET_SPEED")));

    impl Proviso<Feet> for SetSpeed {
        const PRIORITY: usize = 0;
        fn provide(&self, t: &mut Feet) -> impl IntoFuture<Output = std::ops::ControlFlow<()>> {
            *t = Feet(self.speed);
            ready(std::ops::ControlFlow::Continue(()))
        }
    }
}

pub fn test_combatant() -> Rc<Combatant> {
    use xander_runtime::flow::io::Actor;

    let creature = test_creature();

    Rc::new(Combatant {
        creature,
        initiative_score: Cell::new(0),
        actor: Actor::GM,
        position: Cell::new(Position {
            x: Squares(0),
            y: Squares(0),
        }),
        affiliation: Affiliation::default(),
    })
}

pub fn test_creature() -> Rc<Creature> {
    use self::{
        monster::{Cr, Monster},
        stat_block::{AbilityScores, base_score as base_score_},
    };
    use crate::engine::game::{
        creature::{actions::Actions, monster::MonsterType},
        health::Health,
        stats::AbilityScore,
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
            proficiency_bonus: stat_block::defaults::proficiency_bonus(me.clone()),
            proficiencies: stat_block::defaults::proficiencies(),
            scores: AbilityScores {
                str: base_score(9),
                dex: base_score(6),
                con: base_score(8),
                int: base_score(10),
                wis: base_score(6),
                cha: base_score(12),
            },
            modifiers: stat_block::defaults::modifiers(me.clone()),
            health: Health::with_set_max(me.clone(), 7).unwrap(),
            actions: {
                let mut attacks = Actions::new(me.clone());
                attacks
                    .attacks
                    .attacks
                    .get_mut()
                    .push(Rc::new(test_attack("Club")));
                attacks
            },
            reaction: stat_block::defaults::reaction(me),
            speed: {
                let mut provided = Provided::new();
                provided.enroll_mut(provisos::SetSpeed { speed: 30 });
                provided
            },
            ac: {
                let mut provided = Provided::new();
                provided.enroll_mut(SetAc(d20::DExpr::from(6)));
                provided
            },
            markers: stat_block::defaults::markers(),
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

    use crate::engine::game::creature::{Creature, test_combatant};

    #[test]
    fn test_serialize_and_deserialize() {
        let creature = test_combatant();

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
