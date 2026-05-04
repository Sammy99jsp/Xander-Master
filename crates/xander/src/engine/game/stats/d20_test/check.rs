use std::{future::ready, rc::Rc};

use crate::{
    engine::game::{
        combat::Combatant,
        stats::{
            ability::Ability,
            d20_test::{AmbiguousDC, Dc, TestResult, equals_or_exceeds},
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
    pub dc: Option<Dc>,
}

impl Check {
    pub fn for_skill(skill: Skill, dc: Option<Dc>) -> Self {
        Self {
            ability: skill.ability(),
            prof: Some(Box::new(skill)),
            dc,
        }
    }
}

impl Combatant {
    pub async fn check(self: &Rc<Self>, check: Check) -> Outcome<events::PostResultCheckEvent> {
        check.perform(self).await
    }
}

impl D20Test for Check {
    type Target = Dc;
    type Result = CheckResult;
    type Ambiguity = AmbiguousDC;
    type PreRoll = events::PreRollCheckEvent;
    type PreResult = events::PreResultCheckEvent;
    type PostResult = events::PostResultCheckEvent;

    fn base(&self) -> impl IntoFuture<Output = D20TestBase<'_, Self>> {
        ready(D20TestBase {
            ability: &self.ability,
            prof: self.prof.as_deref(),
            target: self.dc.clone().ok_or(AmbiguousDC),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CheckResult {
    Pass,
    Fail,
}

impl TestResult<Check> for CheckResult {
    fn result_for(dc: &d20::ValTree, roll_result: &d20::ValTree) -> CheckResult {
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

#[cfg(test)]
mod tests {
    use std::{future::ready, rc::Rc};

    use xander_runtime::{
        flow::{dispatcher::DispatchState, event::EventHandler, io::TestInterface},
        register, ui,
    };

    use crate::engine::game::{
        Dispatcher, Game,
        combat::arena::Arena,
        creature::{Me, test_combatant},
        stats::{
            d20_test::{Advantage, D20TestRoll, Dc, check::Check},
            skill::{Skill, profs::SkillProficiency},
        },
    };

    #[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    struct Expertise {
        me: Me,
        skill: Skill,
    }

    register!(Expertise, register(Identity("TEST::EXPERTISE"), Lived(@always)));
    impl ui::Ui for Expertise {}

    impl EventHandler<Game> for Expertise {
        type Event = super::events::PreRollCheckEvent;

        fn handle<'s, 'e: 's>(
            &'s self,
            event: &'e mut Self::Event,
        ) -> impl IntoFuture<Output = ()> + 's {
            // If it isn't 'me' making the check...
            if !self.me.is(&event.creature) {
                return ready(());
            }

            // If the check is checking against something 'me' has Expertise in...
            if let Some(prof) = event.check.prof.as_deref()
                && prof.contains(&self.skill)
            {
                // Double the proficiency bonus, and label it with 'Expertise'.
                event.prof_bonus = event
                    .prof_bonus
                    .take()
                    .map(|prof_bonus| (prof_bonus * 2).label(Rc::new(self.clone())));
            }

            ready(())
        }
    }

    #[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct AlwaysInspired {
        me: Me,
    }

    impl ui::Ui for AlwaysInspired {}
    register!(
        AlwaysInspired,
        register(Identity("TEST::ALWAYS_INSPIRED"), Lived(@always))
    );

    impl EventHandler<Game> for AlwaysInspired {
        type Event = super::events::PreRollCheckEvent;

        fn handle<'s, 'e: 's>(
            &'s self,
            event: &'e mut Self::Event,
        ) -> impl IntoFuture<Output = ()> + 's {
            // If it isn't 'me' making the check...
            if !self.me.is(&event.creature) {
                return ready(());
            }

            event.test_dice = event.test_dice.clone().grant(Advantage { reason: None });
            ready(())
        }
    }

    #[test]
    fn check() {
        let game = Game::new(TestInterface, Arena::test());
        let combatant = test_combatant();

        combatant
            .creature
            .stats
            .proficiencies
            .insert(SkillProficiency {
                skill: Skill::Stealth,
            });
        combatant
            .creature
            .stats
            .proficiencies
            .insert(SkillProficiency {
                skill: Skill::Intimidation,
            });

        smol::block_on(async move {
            game.dispatcher
                .dispatch(async move {
                    let game = Dispatcher::local().await;

                    game.listen(Expertise {
                        me: combatant.creature.me(),
                        skill: Skill::Stealth,
                    });

                    game.listen(AlwaysInspired {
                        me: combatant.creature.me(),
                    });

                    println!();

                    let check = Check::for_skill(
                        Skill::Stealth,
                        Some(Dc(<d20::DExpr as From<i32>>::from(15))),
                    );

                    let result = combatant.check(check).await;
                    println!("{result:?}");
                    println!(
                        "Total: {}",
                        result.into_result().unwrap().roll_result.total()
                    );
                })
                .await
        });
    }
}
