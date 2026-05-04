use std::rc::{Rc, Weak};

use crate::engine::game::{
    creature::Creature,
    stats::{
        ability::Ability,
        d20_test::{
            Advantage, D20Test, Disadvantage, PostResultPayload, PreResultPayload, PreRollPayload,
            TargetNumber, TestResult, equals_or_exceeds, utils,
        },
        proficiency::ProficiencyApplicationBase,
    },
};

use super::D20TestBase;

#[derive(Debug)]
pub struct AttackRoll {
    pub against: Weak<Creature>,
    pub ability: Ability,
    pub prof: Option<Box<dyn ProficiencyApplicationBase>>,
}

impl D20Test for AttackRoll {
    type Target = Ac;
    type Result = AttackRollResult;
    type Ambiguity = !;
    type PreRoll = events::PreAttackRollEvent;
    type PreResult = events::PreAttackRollResultEvent;
    type PostResult = events::PostAttackRollResultEvent;

    fn base(&self) -> impl IntoFuture<Output = D20TestBase<'_, Self>> {
        async {
            D20TestBase {
                ability: &self.ability,
                prof: self.prof.as_deref(),
                target: Ok({
                    let against: Rc<Creature> = self.against.upgrade().unwrap();
                    Ac(against.stats.ac.provide(d20::DExpr::ZERO).await)
                }),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ac<T = d20::DExpr>(pub T);

impl Default for Ac {
    fn default() -> Self {
        Self(d20::DExpr::ZERO)
    }
}

impl TargetNumber for Ac {
    fn get(&self) -> &d20::DExpr {
        &self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AttackRollResult {
    Hit(Option<Critical>),
    Miss,
}

impl TestResult<AttackRoll> for AttackRollResult {
    // TODO: Test this logic!
    fn result_for(ac: &d20::ValTree, roll_result: &d20::ValTree) -> AttackRollResult {
        let is_hit = equals_or_exceeds(roll_result, ac);

        let raw_roll = Option::xor(
            roll_result
                .find_labelled_val::<Advantage>()
                .map(d20::ValTree::total),
            roll_result
                .find_labelled_val::<Disadvantage>()
                .map(d20::ValTree::total),
        )
        .unwrap_or_else(|| {
            roll_result
                .find_labelled_val::<utils::TestRoll>()
                .map(d20::ValTree::total)
                .expect("an Xd20 roll should be present on D20Test rolls")
        });

        match raw_roll {
            ..1 => unreachable!(),

            // Natural 1 => Instant Miss
            1 => Self::Miss,

            // Decide normally (accounting for advantage/disadvantage)
            2..20 => match is_hit {
                true => AttackRollResult::Hit(None),
                false => AttackRollResult::Miss,
            },

            // Natural 20 => Hit (Critical)
            20 => Self::Hit(Some(Critical)),

            21.. => unimplemented!("Only d20s in D20Test rolls should be labelled 'TestRoll'!"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Critical;

pub mod events {
    use std::future::ready;

    use xander_runtime::{
        flow::{Event, event::EventBase},
        register,
    };

    use super::*;
    use crate::engine::game::{Game, creature::Creature, stats::ability::AbilityModifier};

    #[derive(Debug)]
    pub struct PreAttackRollEvent {
        pub attack_roll: AttackRoll,
        pub test_dice: d20::DExpr,
        pub ability_modifier: AbilityModifier,
        pub prof_bonus: Option<d20::DExpr>,
        pub circumstantial: Option<d20::DExpr>,
        pub attacker: Weak<Creature>,
    }

    register!(PreAttackRollEvent: dyn EventBase<Game>, register(Identity("ATTACK_ROLL::PRE_ROLL")));
    impl Event<Game> for PreAttackRollEvent {
        type Resolved = PreRollPayload<AttackRoll>;

        fn map_resolved(self) -> impl IntoFuture<Output = Self::Resolved> {
            ready(PreRollPayload {
                test: self.attack_roll,
                creature: self.attacker,
                test_dice: self.test_dice,
                ability_modifier: self.ability_modifier,
                prof_bonus: self.prof_bonus,
                circumstantial: self.circumstantial,
            })
        }

        type Cancelled = !;

        fn map_cancelled(self) -> impl IntoFuture<Output = Self::Cancelled> {
            async { todo!() }
        }
    }

    impl From<PreRollPayload<AttackRoll>> for PreAttackRollEvent {
        fn from(payload: PreRollPayload<AttackRoll>) -> Self {
            Self {
                attack_roll: payload.test,
                test_dice: payload.test_dice,
                ability_modifier: payload.ability_modifier,
                prof_bonus: payload.prof_bonus,
                circumstantial: payload.circumstantial,
                attacker: payload.creature,
            }
        }
    }

    #[derive(Debug)]
    pub struct PreAttackRollResultEvent {
        pub attack_roll: AttackRoll,
        pub roll_result: d20::ValTree,
        pub attacker: Weak<Creature>,
    }

    register!(PreAttackRollResultEvent: dyn EventBase<Game>, register(Identity("ATTACK_ROLL::PRE_RESULT")));
    impl Event<Game> for PreAttackRollResultEvent {
        type Resolved = PreResultPayload<AttackRoll>;

        fn map_resolved(self) -> impl IntoFuture<Output = Self::Resolved> {
            ready(PreResultPayload {
                test: self.attack_roll,
                creature: self.attacker,
                roll_result: self.roll_result,
            })
        }

        type Cancelled = !;

        fn map_cancelled(self) -> impl IntoFuture<Output = Self::Cancelled> {
            async { todo!() }
        }
    }

    impl From<PreResultPayload<AttackRoll>> for PreAttackRollResultEvent {
        fn from(payload: PreResultPayload<AttackRoll>) -> Self {
            Self {
                attack_roll: payload.test,
                roll_result: payload.roll_result,
                attacker: payload.creature,
            }
        }
    }

    #[derive(Debug)]
    pub struct PostAttackRollResultEvent {
        pub attack_roll: AttackRoll,
        pub attack_roll_result: d20::ValTree,
        pub result: super::AttackRollResult,
        pub attacker: Weak<Creature>,
    }

    register!(PostAttackRollResultEvent: dyn EventBase<Game>, register(Identity("ATTACK_ROLL::POST_RESULT")));
    impl Event<Game> for PostAttackRollResultEvent {
        type Resolved = Self;

        fn map_resolved(self) -> impl IntoFuture<Output = Self::Resolved> {
            ready(self)
        }

        type Cancelled = !;

        fn map_cancelled(self) -> impl IntoFuture<Output = Self::Cancelled> {
            async { todo!() }
        }
    }

    impl From<PostResultPayload<AttackRoll>> for PostAttackRollResultEvent {
        fn from(payload: PostResultPayload<AttackRoll>) -> Self {
            Self {
                attack_roll: payload.test,
                attack_roll_result: payload.roll_result,
                result: payload.test_result,
                attacker: payload.creature,
            }
        }
    }
}

pub mod provisos {
    use std::{future::ready, ops::ControlFlow};

    use xander_runtime::lived::provided::prelude::*;

    #[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct SetAc(pub d20::DExpr);

    register!(SetAc: dyn ProvisoBase<d20::DExpr>, register(Identity("SET_AC"), Lived(always), Archive, Deserialize));

    impl ArchivedProvisoBase<d20::DExpr> for rkyv::Archived<SetAc> {}
    impl Proviso<d20::DExpr> for SetAc {
        const PRIORITY: usize = 0; // We're first!

        fn provide(&self, t: &mut d20::DExpr) -> impl IntoFuture<Output = ControlFlow<()>> {
            *t = self.0.clone();
            ready(ControlFlow::Continue(()))
        }
    }
}
