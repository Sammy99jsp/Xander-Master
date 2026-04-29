pub mod character;
pub mod monster;
pub mod proficiencies;
pub mod stat_block;

use std::rc::{Rc, Weak};

use xander_runtime::flow::io::Actor;

use crate::engine::game::creature::stat_block::StatBlock;

#[derive(rkyv::Archive, rkyv::Serialize, Debug)]
pub struct Creature {
    pub id: CreatureId,
    pub name: String,
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

    pub fn deserialize<E>() -> Rc<Self> {
        todo!()
    }
}

/// [Me] is a serializable/deserializable (weak) reference to a creature
/// with a guarantee that there is always at least (one) strong reference
/// to the creature.
///
/// Use this type for things that are self-referential to the creature
/// that holds a value.
///
/// For external creatures, you may use the usual Weak<Creature>.
///
/// This type is primarily to allow for cyclical references within the [rkyv] ecosystem.
///
#[derive(Clone)]
pub struct Me(Weak<Creature>);

impl std::fmt::Debug for Me {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Me").finish()
    }
}

pub mod ui {
    use xander_runtime::ui;

    impl ui::UI for super::Creature {}
}

pub mod prov {
    use std::future::ready;

    use dynx::Identity;
    use xander_runtime::{
        always_alive,
        lived::provided::{ArchivedProvisoBase, Proviso, ProvisoBase},
        register,
    };

    use crate::engine::game::{creature::Me, stats::proficiency::ProficiencyBonus};

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

// Archiving

pub mod archiving {
    use std::{
        mem::MaybeUninit,
        ops::Deref,
        rc::{Rc, Weak},
    };

    use dynx::dynx::DynDeserializer;
    use rkyv::{
        Archive, Deserialize, Serialize,
        de::Pooling,
        rancor::{Fallible, Source},
    };

    use crate::engine::game::creature::{CreatureId, Me};

    use super::Creature;

    impl Deref for Me {
        type Target = Creature;

        fn deref(&self) -> &Self::Target {
            if self.0.strong_count() == 0 {
                panic!("Tried to dereference me without any strong references!");
            }

            unsafe { self.0.as_ptr().as_ref().unwrap() }
        }
    }

    impl Archive for Me {
        type Archived = rkyv::Archived<CreatureId>;
        type Resolver = rkyv::Resolver<CreatureId>;

        fn resolve(&self, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
            self.id.resolve(resolver, out);
        }
    }

    impl<S> Serialize<S> for Me
    where
        S: Fallible + ?Sized,
    {
        fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, <S as Fallible>::Error> {
            self.id.serialize(serializer)
        }
    }

    impl<D> Deserialize<Me, D> for rkyv::Archived<Me>
    where
        D: Fallible + ?Sized,
    {
        fn deserialize(&self, deserializer: &mut D) -> Result<Me, <D as Fallible>::Error> {
            // SAFETY: We should only be called by something that is owned by Creature...
            let deserializer = unsafe {
                (deserializer as *mut D as *mut Deserializer<'_, dyn DynDeserializer>)
                    .as_mut()
                    .unwrap_unchecked()
            };

            Ok(Me(deserializer.creature(self.to_id())))
        }
    }

    pub trait CreatureDeserializer: Fallible {
        fn creature(&self, id: CreatureId) -> Weak<Creature>;
    }

    pub struct Deserializer<'a, D: ?Sized> {
        creature: Rc<MaybeUninit<Creature>>,
        inner: &'a mut D,
    }

    impl<'a, D> CreatureDeserializer for Deserializer<'a, D>
    where
        D: Fallible + ?Sized,
    {
        fn creature(&self, _: CreatureId) -> Weak<Creature> {
            let weak = Rc::downgrade(&self.creature);

            unsafe { Weak::from_raw(Weak::into_raw(weak).cast::<Creature>()) }
        }
    }

    impl<'a, D> Fallible for Deserializer<'a, D>
    where
        D: Fallible + ?Sized,
    {
        type Error = D::Error;
    }

    impl<'a, D> Pooling for Deserializer<'a, D>
    where
        D: Fallible + Pooling + ?Sized,
    {
        #[inline]
        fn start_pooling(&mut self, address: usize) -> rkyv::de::PoolingState {
            self.inner.start_pooling(address)
        }

        unsafe fn finish_pooling(
            &mut self,
            address: usize,
            ptr: rkyv::de::ErasedPtr,
            drop: unsafe fn(rkyv::de::ErasedPtr),
        ) -> Result<(), <Self as Fallible>::Error> {
            unsafe { self.inner.finish_pooling(address, ptr, drop) }
        }
    }

    impl<D> Deserialize<Rc<Creature>, D> for rkyv::Archived<Creature>
    where
        D: Fallible + Pooling + ?Sized,
        D::Error: Source,
    {
        fn deserialize(
            &self,
            deserializer: &mut D,
        ) -> Result<Rc<Creature>, <D as Fallible>::Error> {
            let mut deserializer = Deserializer {
                creature: Rc::new_uninit(),
                inner: deserializer,
            };

            let ptr = deserializer.creature.as_ptr().cast_mut();

            unsafe {
                std::ptr::write(
                    ptr,
                    Creature {
                        id: {
                            println!("Deserializing id");
                            self.id.deserialize(&mut deserializer)?
                        },
                        name: self.name.deserialize(&mut deserializer)?,
                        kind: {
                            println!("Deserializing kind");
                            self.kind.deserialize(&mut deserializer)?
                        },
                        stats: {
                            println!("Deserializing stats");
                            self.stats.deserialize(&mut deserializer)?
                        },
                    },
                )
            };

            unsafe { Ok(deserializer.creature.assume_init()) }
        }
    }

    impl super::ArchivedCreatureId {
        fn to_id(&self) -> CreatureId {
            CreatureId(self.0.to_native())
        }
    }
}

#[cfg(test)]
pub fn test_creature() -> Rc<Creature> {
    use self::{
        monster::{Cr, Monster},
        proficiencies::Proficiencies,
        prov::CreatureProficiencyBonus,
        stat_block::{AbilityModifiers, AbilityScores, base_score as base_score_},
    };
    use crate::engine::game::{health::Health, stats::AbilityScore};
    use xander_runtime::lived::Provided;

    fn base_score(s: u8) -> Provided<AbilityScore> {
        base_score_(AbilityScore::try_from(s).unwrap())
    }

    Creature::new(|me| Creature {
        id: CreatureId(0),
        name: "Test-Creature".to_string(),
        kind: CreatureKind::Monster(Monster { cr: Cr::Half }),
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
            health: Health {
                temp_hp: Default::default(),
            },
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
        let result = archived
            .get()
            .deserialize(deserializer as &mut dyn DynDeserializer)
            .unwrap();

        println!("{result:?}")
    }
}
