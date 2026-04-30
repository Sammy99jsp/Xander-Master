use std::rc::Rc;

use crate::{
    engine::game::{
        creature::Creature,
        stats::{
            ability::Ability,
            d20_test::{
                AmbiguousDC, D20Test, DC, PostResultPayload, PreResultPayload, PreRollPayload,
                TestResult, equals_or_exceeds,
            },
            proficiency::ProficiencyApplicationBase,
        },
    },
    prelude::event::Outcome,
};

use super::D20TestBase;

#[derive(Debug)]
pub struct Save {
    pub ability: Ability,
    pub prof: Option<Box<dyn ProficiencyApplicationBase>>,
    pub dc: Option<DC>,
}

impl Creature {
    pub async fn save(self: &Rc<Self>, save: Save) -> Outcome<events::PostResultSaveEvent> {
        save.perform(self).await
    }
}

impl D20Test for Save {
    type Target = DC;
    type Ambiguity = AmbiguousDC;
    type Result = SaveResult;
    type PreRoll = events::PreRollSaveEvent;
    type PreResult = events::PreResultSaveEvent;
    type PostResult = events::PostResultSaveEvent;

    fn base(&self) -> D20TestBase<'_, Self> {
        D20TestBase {
            ability: &self.ability,
            prof: self.prof.as_deref(),
            target: self.dc.as_ref().ok_or(AmbiguousDC),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SaveResult {
    Pass,
    Fail,
}

impl TestResult<Save> for SaveResult {
    fn result_for(dc: &DC, roll_result: &d20::ValTree) -> SaveResult {
        match equals_or_exceeds(roll_result, dc) {
            true => SaveResult::Pass,
            false => SaveResult::Fail,
        }
    }
}

pub mod profs {
    use crate::{engine::game::stats::ability::Ability, prelude::proficiency::*};

    #[derive(Debug, Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct SaveProficiency {
        pub ability: Ability,
    }

    register!(SaveProficiency: dyn ProficiencyBase, register(Archive, Deserialize, Identity("SAVE"), Lived(always)));

    impl ArchivedProficiencyBase for rkyv::Archived<SaveProficiency> {}

    impl Proficiency for SaveProficiency {
        type Application = Ability;

        fn applies_to(&self, app: &Self::Application) -> bool {
            self.ability == *app
        }
    }
}

pub mod decisions {
    use std::rc::Weak;

    use xander_runtime::{flow::decision::prelude::*, ui};

    use crate::engine::game::{
        creature::Creature,
        stats::d20_test::{AmbiguousDC, Disambiguation},
    };

    use super::Save;

    impl Disambiguation<Save> for AmbiguousDC {
        type Decision = GMDecidesSaveResult;

        fn disambiguate(
            save: Weak<Save>,
            creature: Weak<Creature>,
            roll_result: d20::ValTree,
        ) -> Self::Decision {
            GMDecidesSaveResult {
                save,
                creature,
                roll_result,
            }
        }
    }

    #[derive(Debug)]
    pub struct GMDecidesSaveResult {
        save: Weak<Save>,
        creature: Weak<Creature>,
        roll_result: d20::ValTree,
    }

    register!(GMDecidesSaveResult: Decision, register(Identity("SAVE::GM_DECIDES")));
    impl IntoDecision for GMDecidesSaveResult {
        type Response = super::SaveResult;
        type Kind = Selection;

        fn into_decision(self) -> Decision {
            Decision::new::<Self>(
                vec![Actor::GM],
                ui::Component::Multi(vec![
                    ui::Component::Text("Please rule on whether ".to_string()),
                    ui::Component::RefRich(self.creature),
                    ui::Component::Text(" passes or fails the ".to_string()),
                    ui::Component::RefRich(self.save),
                    ui::Component::Text(" save with: ".to_string()),
                    ui::Component::Rich(Box::new(self.roll_result)),
                ]),
                Selection {
                    items: vec![
                        Box::new(super::SaveResult::Pass),
                        Box::new(super::SaveResult::Fail),
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

    use xander_runtime::register;

    use crate::engine::game::{creature::Creature, stats::ability::AbilityModifier};
    use crate::prelude::event::*;

    use super::{PostResultPayload, PreResultPayload, PreRollPayload, Save};

    #[derive(Debug)]
    pub struct PreRollSaveEvent {
        pub save: Save,
        pub test_dice: d20::DExpr,
        pub ability_modifier: AbilityModifier,
        pub prof_bonus: Option<d20::DExpr>,
        pub circumstantial: Option<d20::DExpr>,
        pub creature: Weak<Creature>,
    }

    register!(PreRollSaveEvent: dyn EventBase<Game>, register(Identity("SAVE::PRE_ROLL")));
    impl Event<Game> for PreRollSaveEvent {
        type Resolved = PreRollPayload<Save>;

        fn map_resolved(self) -> impl IntoFuture<Output = Self::Resolved> {
            ready(PreRollPayload {
                test: self.save,
                creature: self.creature,
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

    impl From<PreRollPayload<Save>> for PreRollSaveEvent {
        fn from(payload: PreRollPayload<Save>) -> Self {
            Self {
                save: payload.test,
                test_dice: payload.test_dice,
                ability_modifier: payload.ability_modifier,
                prof_bonus: payload.prof_bonus,
                circumstantial: payload.circumstantial,
                creature: payload.creature,
            }
        }
    }

    #[derive(Debug)]
    pub struct PreResultSaveEvent {
        pub save: Save,
        pub roll_result: d20::ValTree,
        pub creature: Weak<Creature>,
    }

    register!(PreResultSaveEvent: dyn EventBase<Game>, register(Identity("SAVE::PRE_RESULT")));
    impl Event<Game> for PreResultSaveEvent {
        type Resolved = PreResultPayload<Save>;

        fn map_resolved(self) -> impl IntoFuture<Output = Self::Resolved> {
            ready(PreResultPayload {
                test: self.save,
                roll_result: self.roll_result,
                creature: self.creature,
            })
        }

        type Cancelled = !;

        fn map_cancelled(self) -> impl IntoFuture<Output = Self::Cancelled> {
            async { todo!() }
        }
    }

    impl From<PreResultPayload<Save>> for PreResultSaveEvent {
        fn from(payload: PreResultPayload<Save>) -> Self {
            Self {
                save: payload.test,
                roll_result: payload.roll_result,
                creature: payload.creature,
            }
        }
    }

    #[derive(Debug)]
    pub struct PostResultSaveEvent {
        pub save: Save,
        pub roll_result: d20::ValTree,
        pub result: super::SaveResult,
        pub creature: Weak<Creature>,
    }

    register!(PostResultSaveEvent: dyn EventBase<Game>, register(Identity("SAVE::POST_RESULT")));
    impl Event<Game> for PostResultSaveEvent {
        type Resolved = Self;

        fn map_resolved(self) -> impl IntoFuture<Output = Self::Resolved> {
            ready(self)
        }

        type Cancelled = !;

        fn map_cancelled(self) -> impl IntoFuture<Output = Self::Cancelled> {
            async { todo!() }
        }
    }

    impl From<PostResultPayload<Save>> for PostResultSaveEvent {
        fn from(payload: PostResultPayload<Save>) -> Self {
            Self {
                save: payload.test,
                roll_result: payload.roll_result,
                result: payload.test_result,
                creature: payload.creature,
            }
        }
    }
}

pub mod ui {
    use xander_runtime::ui;

    impl ui::Ui for super::SaveResult {}
    impl ui::Ui for super::Save {}
}
