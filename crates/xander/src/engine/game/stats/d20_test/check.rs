use std::rc::Rc;

use crate::{
    engine::game::{
        creature::Creature,
        stats::{
            ability::Ability,
            d20_test::{AmbiguousDC, DC, TestResult, equals_or_exceeds},
            proficiency::ProficiencyApplicationBase,
            skill::Skill,
        },
    },
    prelude::event::Outcome,
};

pub use super::{
    D20Test, D20TestBase, Disambiguation, PostResultPayload, PreResultPayload, PreRollPayload,
};

#[derive(Debug, Clone)]
pub struct Check {
    pub ability: Ability,
    pub prof: Option<Box<dyn ProficiencyApplicationBase>>,
    pub dc: Option<DC>,
}

impl Check {
    pub fn for_skill(skill: Skill, dc: Option<DC>) -> Self {
        Self {
            ability: skill.ability(),
            prof: Some(Box::new(skill)),
            dc,
        }
    }
}

impl Creature {
    pub async fn check(self: &Rc<Self>, check: Check) -> Outcome<events::PostResultCheckEvent> {
        check.perform(self).await
    }
}

impl D20Test for Check {
    type Target = DC;
    type Result = CheckResult;
    type Ambiguity = AmbiguousDC;
    type PreRoll = events::PreRollCheckEvent;
    type PreResult = events::PreResultCheckEvent;
    type PostResult = events::PostResultCheckEvent;

    fn base(&self) -> D20TestBase<'_, Self> {
        D20TestBase {
            ability: &self.ability,
            prof: self.prof.as_deref(),
            target: self.dc.as_ref().ok_or(AmbiguousDC),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CheckResult {
    Pass,
    Fail,
}

impl TestResult<Check> for CheckResult {
    fn result_for(dc: &DC, roll_result: &d20::ValTree) -> CheckResult {
        match equals_or_exceeds(roll_result, dc) {
            true => CheckResult::Pass,
            false => CheckResult::Fail,
        }
    }
}

pub mod decisions {
    use std::rc::Weak;

    use xander_runtime::{flow::decision::prelude::*, ui};

    use crate::engine::game::{creature::Creature, stats::d20_test::Check};

    #[derive(Debug)]
    pub struct GMDecidesCheckResult {
        pub creature: Weak<Creature>,
        pub check: Weak<super::Check>,
        pub roll_result: d20::ValTree,
    }

    impl super::Disambiguation<super::Check> for super::AmbiguousDC {
        type Decision = GMDecidesCheckResult;

        fn disambiguate(
            test: Weak<Check>,
            creature: Weak<Creature>,
            roll_result: d20::ValTree,
        ) -> Self::Decision {
            GMDecidesCheckResult {
                creature,
                check: test,
                roll_result,
            }
        }
    }

    register!(GMDecidesCheckResult: Decision, register(Identity("CHECK::GM_DECIDES")));
    impl IntoDecision for GMDecidesCheckResult {
        type Response = super::CheckResult;
        type Kind = Selection;

        fn into_decision(self) -> Decision {
            Decision::new::<Self>(
                vec![Actor::GM],
                ui::Component::Multi(vec![
                    ui::Component::Text("Please rule on whether ".to_string()),
                    ui::Component::RefRich(self.creature),
                    ui::Component::Text(" passes or fails the ".to_string()),
                    ui::Component::RefRich(self.check),
                    ui::Component::Text(" check with: ".to_string()),
                    ui::Component::Rich(Box::new(self.roll_result)),
                ]),
                Selection {
                    items: vec![
                        Box::new(super::CheckResult::Pass),
                        Box::new(super::CheckResult::Fail),
                    ],
                    validate: None,
                    qty: 1,
                },
            )
        }
    }
}

pub mod events {
    use std::{future::ready, rc::Weak};

    use xander_runtime::{
        cancellable,
        flow::{Event, event::EventBase},
        register, ui,
    };

    use crate::engine::game::{Game, creature::Creature, stats::ability::AbilityModifier};

    use super::{Check, PostResultPayload, PreResultPayload, PreRollPayload};

    #[derive(Debug)]
    pub struct CheckCancelled {
        pub reason: Weak<dyn ui::Ui>,
        pub result: Option<super::CheckResult>,
    }

    #[derive(Debug)]
    pub struct PreRollCheckEvent {
        pub check: Check,
        pub test_dice: d20::DExpr,
        pub ability_modifier: AbilityModifier,
        pub prof_bonus: Option<d20::DExpr>,
        pub circumstantial: Option<d20::DExpr>,
        pub creature: Weak<Creature>,
        pub cancelled: Option<CheckCancelled>,
    }

    register!(PreRollCheckEvent: dyn EventBase<Game>, register(Identity("CHECK::PRE_ROLL")));
    impl Event<Game> for PreRollCheckEvent {
        type Resolved = PreRollPayload<Check>;

        fn map_resolved(self) -> impl IntoFuture<Output = Self::Resolved> {
            ready(PreRollPayload {
                test: self.check,
                creature: self.creature,
                test_dice: self.test_dice,
                ability_modifier: self.ability_modifier,
                prof_bonus: self.prof_bonus,
                circumstantial: self.circumstantial,
            })
        }

        type Cancelled = CheckCancelled;

        fn map_cancelled(self) -> impl IntoFuture<Output = Self::Cancelled> {
            ready(self.cancelled.unwrap())
        }
    }

    impl From<PreRollPayload<Check>> for PreRollCheckEvent {
        fn from(payload: PreRollPayload<Check>) -> Self {
            Self {
                check: payload.test,
                test_dice: payload.test_dice,
                ability_modifier: payload.ability_modifier,
                prof_bonus: payload.prof_bonus,
                circumstantial: payload.circumstantial,
                creature: payload.creature,
                cancelled: None,
            }
        }
    }

    cancellable!(PreRollCheckEvent, cancelled);

    #[derive(Debug)]
    pub struct PreResultCheckEvent {
        pub check: Check,
        pub roll_result: d20::ValTree,
        pub creature: Weak<Creature>,
        pub cancelled: Option<CheckCancelled>,
    }

    register!(PreResultCheckEvent: dyn EventBase<Game>, register(Identity("CHECK::PRE_RESULT")));
    impl Event<Game> for PreResultCheckEvent {
        type Resolved = PreResultPayload<Check>;

        fn map_resolved(self) -> impl IntoFuture<Output = Self::Resolved> {
            ready(PreResultPayload {
                test: self.check,
                roll_result: self.roll_result,
                creature: self.creature,
            })
        }

        type Cancelled = CheckCancelled;

        fn map_cancelled(self) -> impl IntoFuture<Output = Self::Cancelled> {
            ready(self.cancelled.unwrap())
        }
    }

    impl From<PreResultPayload<Check>> for PreResultCheckEvent {
        fn from(payload: PreResultPayload<Check>) -> Self {
            Self {
                check: payload.test,
                roll_result: payload.roll_result,
                creature: payload.creature,
                cancelled: None,
            }
        }
    }

    cancellable!(PreResultCheckEvent, cancelled);

    #[derive(Debug)]
    pub struct PostResultCheckEvent {
        pub check: Check,
        pub roll_result: d20::ValTree,
        pub result: super::CheckResult,
        pub creature: Weak<Creature>,
        pub cancelled: Option<CheckCancelled>,
    }

    register!(PostResultCheckEvent: dyn EventBase<Game>, register(Identity("CHECK::POST_RESULT")));
    impl Event<Game> for PostResultCheckEvent {
        type Resolved = Self;

        fn map_resolved(self) -> impl IntoFuture<Output = Self::Resolved> {
            ready(self)
        }

        type Cancelled = CheckCancelled;

        fn map_cancelled(self) -> impl IntoFuture<Output = Self::Cancelled> {
            ready(self.cancelled.unwrap())
        }
    }
    cancellable!(PostResultCheckEvent, cancelled);

    impl From<PostResultPayload<Check>> for PostResultCheckEvent {
        fn from(payload: PostResultPayload<Check>) -> Self {
            Self {
                check: payload.test,
                roll_result: payload.roll_result,
                result: payload.test_result,
                creature: payload.creature,
                cancelled: None,
            }
        }
    }
}

pub mod ui {
    use xander_runtime::ui;

    impl ui::Ui for super::Check {}
    impl ui::Ui for super::CheckResult {}
}

// #[cfg(test)]
// mod tests {
//     use std::{
//         future::ready,
//         rc::{Rc, Weak},
//     };

//     use xander_runtime::{always_alive, flow::event::EventHandler, lived::provided::Provided, ui};

//     use crate::engine::game::{
//         Dispatcher, Game,
//         creature::{
//             Creature, CreatureKind,
//             monster::{Cr, Monster, provisos::MonsterProficiencyBonus},
//             proficiencies::Proficiencies,
//             stat_block::{self, AbilityModifiers, AbilityScores, StatBlock},
//         },
//         health::Health,
//         stats::{
//             ability::AbilityScore,
//             d20_test::{Advantage, D20TestRoll, DC, check::Check},
//             skill::{Skill, profs::SkillProficiency},
//         },
//     };

//     #[derive(Debug, Clone)]
//     struct Expertise {
//         me: Weak<Creature>,
//         skill: Skill,
//     }

//     always_alive!(Expertise);
//     impl ui::Ui for Expertise {}

//     impl EventHandler<Game> for Expertise {
//         type Event = super::events::PreRollCheckEvent;

//         fn handle<'s, 'e: 's>(
//             &'s self,
//             event: &'e mut Self::Event,
//         ) -> impl IntoFuture<Output = ()> + 's {
//             // If it isn't 'me' making the check...
//             if !event.creature.ptr_eq(&self.me) {
//                 return ready(());
//             }

//             // If the check is checking against something 'me' has Expertise in...
//             if let Some(prof) = event.check.prof.as_deref()
//                 && prof.contains(&self.skill)
//             {
//                 // Double the proficiency bonus, and label it with 'Expertise'.
//                 event.prof_bonus = event
//                     .prof_bonus
//                     .take()
//                     .map(|prof_bonus| (prof_bonus * 2).label(Rc::new(self.clone())));
//             }

//             ready(())
//         }
//     }

//     #[derive(Debug)]
//     pub struct AlwaysInspired {
//         me: Weak<Creature>,
//     }

//     always_alive!(AlwaysInspired);
//     impl ui::Ui for AlwaysInspired {}

//     impl EventHandler<Game> for AlwaysInspired {
//         type Event = super::events::PreRollCheckEvent;

//         fn handle<'s, 'e: 's>(
//             &'s self,
//             event: &'e mut Self::Event,
//         ) -> impl IntoFuture<Output = ()> + 's {
//             // If it isn't 'me' making the check...
//             if !event.creature.ptr_eq(&self.me) {
//                 return ready(());
//             }

//             event.test_dice = event.test_dice.clone().grant(Advantage { reason: None });
//             ready(())
//         }
//     }

//     #[test]
//     fn check() {
//         let game = Game::test();
//         let creature = Rc::new_cyclic(|me| Creature {
//             kind: CreatureKind::Monster(Monster {
//                 cr: Cr::try_new(30).unwrap(),
//             }),
//             stats: StatBlock {
//                 me: me.clone(),
//                 proficiency_bonus: {
//                     let mut provided = Provided::new();
//                     provided.enroll_mut(MonsterProficiencyBonus { me: me.clone() });
//                     provided
//                 },
//                 proficiencies: {
//                     let mut profs = Proficiencies::new();
//                     profs.insert_mut(SkillProficiency {
//                         skill: Skill::Stealth,
//                     });
//                     profs.insert_mut(SkillProficiency {
//                         skill: Skill::Intimidation,
//                     });
//                     profs
//                 },
//                 scores: AbilityScores {
//                     str: stat_block::base_score(AbilityScore::try_from(20).unwrap()),
//                     dex: stat_block::base_score(AbilityScore::try_from(20).unwrap()),
//                     con: stat_block::base_score(AbilityScore::try_from(20).unwrap()),
//                     int: stat_block::base_score(AbilityScore::try_from(20).unwrap()),
//                     wis: stat_block::base_score(AbilityScore::try_from(20).unwrap()),
//                     cha: stat_block::base_score(AbilityScore::try_from(20).unwrap()),
//                 },
//                 modifiers: AbilityModifiers::new(me.clone()),
//                 health: Health::new(me.clone()),
//             },
//         });

//         smol::block_on(async move {
//             game.dispatcher
//                 .dispatch(async move {
//                     let game = Dispatcher::local().await;

//                     game.listen(Expertise {
//                         me: Rc::downgrade(&creature),
//                         skill: Skill::Stealth,
//                     });

//                     game.listen(AlwaysInspired {
//                         me: Rc::downgrade(&creature),
//                     });

//                     let check = Check::for_skill(Skill::Stealth, Some(DC(40)));
//                     let result = creature.check(check).await;
//                     println!("{result:#?}");
//                     println!(
//                         "Total: {}",
//                         result.into_result().unwrap().roll_result.total()
//                     );
//                 })
//                 .await
//         });
//     }
// }
