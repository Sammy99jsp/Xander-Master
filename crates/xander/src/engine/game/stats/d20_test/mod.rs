use std::{
    cmp::Ordering,
    marker::PhantomData,
    rc::{Rc, Weak},
};

use xander_runtime::{
    DynWeak,
    flow::{
        Event, Interface,
        decision::Response,
        dispatcher::DispatchState,
        io::{Decision, IntoDecision},
    },
    identity,
};

use crate::{
    engine::{
        game::{
            Dispatcher, Game,
            creature::Creature,
            stats::{
                ability::{Ability, AbilityModifier},
                proficiency::ProficiencyApplicationBase,
            },
        },
        io::User,
    },
    prelude::event::Outcome,
};

pub mod attack_roll;
pub mod check;
pub mod save;
pub mod utils;

pub use check::{Check, CheckResult};
pub use utils::{Advantage, D20TestRoll, Disadvantage};

/// Give a specially labelled D20 for Checks, Saves, and Attack Rolls.
/// This is used as a label for [d20::Dice] to know where to grant/impose [Advantage]/[Disadvantage]
fn d20_test() -> d20::DExpr {
    d20::D20.label(Rc::new(utils::TestRoll))
}

pub(super) fn equals_or_exceeds<T: TargetNumber>(roll_result: &d20::ValTree, target: &T) -> bool {
    matches!(
        roll_result.total().cmp(&(*target).into()),
        Ordering::Equal | Ordering::Greater
    )
}

/// Common state between all [D20Test] impls.
pub struct D20TestBase<'a, Test>
where
    Test: D20Test,
{
    pub(super) ability: &'a Ability,
    pub(super) prof: Option<&'a dyn ProficiencyApplicationBase>,
    pub(super) target: Result<&'a Test::Target, Test::Ambiguity>,
}

/// Common state between all [D20Test::PreRoll] impls.
pub struct PreRollPayload<Test>
where
    Test: D20Test,
{
    pub test: Test,

    pub creature: Weak<Creature>,
    pub test_dice: d20::DExpr,
    pub ability_modifier: AbilityModifier,
    pub prof_bonus: Option<d20::DExpr>,
    pub circumstantial: Option<d20::DExpr>,
}

/// Common state between all [D20Test::PreResult] impls.
pub struct PreResultPayload<Test>
where
    Test: D20Test,
{
    pub test: Test,

    pub creature: Weak<Creature>,
    pub roll_result: d20::ValTree,
}

/// Common state between all [D20Test::PostResult] impls.
pub struct PostResultPayload<Test>
where
    Test: D20Test,
{
    pub test: Test,

    pub creature: Weak<Creature>,
    pub roll_result: d20::ValTree,
    pub test_result: Test::Result,
}

/// Shared logic between all D20 Tests (pg. 6, SRD 5.2E).
///
/// A D20 Test determines the fate of any uncertain action.
/// Namely, they are:
/// - [check::Check]
/// - [save::Save]
/// - [attack_roll::AttackRoll]
pub trait D20Test: Sized {
    /// See [TargetNumber]
    type Target: TargetNumber;

    /// See [TestResult]
    type Result: TestResult<Self>;

    /// See [Disambiguation]
    type Ambiguity: Disambiguation<Self>;

    /// Event fired before any rolling has occurred.
    ///
    /// Any Advantage/Disadvantage, or circumstantial modifiers are handled here.
    /// This could include:
    /// - Exhaustion
    /// - Heroic Inspiration
    /// - Experience
    /// - Jack of All Trades
    type PreRoll: From<PreRollPayload<Self>>
        + Event<
            Game,
            Resolved = PreRollPayload<Self>,
            Cancelled = <Self::PostResult as Event<Game>>::Cancelled,
        >;

    /// Event fired just after rolling has occurred, but before the result of the test
    /// is known (evaluated).
    ///
    /// This is for features that allow players to modify the total roll
    /// just before the result is announced by the DM, such as:
    /// - Bardic Inspiration
    type PreResult: From<PreResultPayload<Self>>
        + Event<
            Game,
            Resolved = PreResultPayload<Self>,
            Cancelled = <Self::PostResult as Event<Game>>::Cancelled,
        >;

    /// Event fired after the result is known, at the end of the process.
    type PostResult: From<PostResultPayload<Self>> + Event<Game>;

    /// See [D20TestBase].
    #[doc(hidden)]
    fn base(&self) -> D20TestBase<'_, Self>;

    fn perform(self, me: &Rc<Creature>) -> impl IntoFuture<Output = Outcome<Self::PostResult>> {
        async {
            // 4. "Roll 1d20"
            let test_dice = d20_test();

            // 5. "Add Modifiers"
            let D20TestBase {
                ability,
                prof,
                target,
            } = self.base();

            let (ability, target) = (*ability, target.copied());

            // 5.1 "The Relevant Ability Modifier"
            let ability_modifier = me.stats.modifiers.get(ability).await;

            // 5.2 "Your Proficiency Bonus (If Relevant)"
            //      If we have a relevant proficiency, apply the bonus
            let prof_bonus = if let Some(with) = prof
                && let Some(prof) = me.stats.proficiencies.get(with).await
            {
                let bonus = me.stats.proficiency_bonus.get().await;
                // We'll keep track of the "how" and label to modifier.
                Some(bonus.into_expr(DynWeak::new(Rc::downgrade(me)), DynWeak::new(prof)))
            } else {
                None
            };

            // 5.3 "Circumstantial Bonuses and Penalties"
            //      Any downstream event handlers will do something with this Option<..>
            let circumstantial = None;

            // Fire the pre-roll event.
            //
            let PreRollPayload {
                test,
                test_dice,
                ability_modifier,
                prof_bonus,
                circumstantial,
                creature,
            } = Self::PreRoll::from(PreRollPayload {
                test: self,
                creature: Rc::downgrade(me),
                test_dice,
                ability_modifier,
                prof_bonus,
                circumstantial,
            })
            .handle()
            .await?;

            // 6. "Compare the total to a Target Number"

            let mut roll = test_dice + ability_modifier.into_expr(ability, Rc::downgrade(me));

            if let Some(prof_bonus) = prof_bonus {
                roll += prof_bonus;
            }

            if let Some(circumstantial) = circumstantial {
                roll += circumstantial;
            }

            // Roll for the check, including all bonuses/penalties.

            let game = Dispatcher::local().await;
            let User { roller, .. } = game.interface().state_for(me.actor());

            let roll_result = roller
                .roll(&roll)
                .await
                .expect("valid dice expr, no errors whilst rolling");
            // Fire the pre-result event
            //
            let PreResultPayload {
                test,
                creature,
                roll_result,
            } = Self::PreResult::from(PreResultPayload {
                test,
                creature,
                roll_result,
            })
            .handle()
            .await?;

            let (test, test_result) = match target {
                Ok(set_target) => {
                    // We can finally compare the result to the target number.
                    let test_result =
                        <Self::Result as TestResult<Self>>::result_for(&set_target, &roll_result);

                    (test, test_result)
                }

                // Ambiguous target number (DC)
                // We must clarify with the DM.
                Err(_) => {
                    let test_rc = Rc::new(test);
                    let test_result = <Self::Ambiguity as Disambiguation<Self>>::disambiguate(
                        Rc::downgrade(&test_rc),
                        creature.clone(),
                        roll_result.clone(),
                    )
                    .decide::<Game>()
                    .await;

                    (Rc::into_inner(test_rc).unwrap(), test_result)
                }
            };

            Self::PostResult::from(PostResultPayload {
                test,
                creature,
                roll_result,
                test_result,
            })
            .handle()
            .await
        }
    }
}

/// The target number for a test is compared with a test roll
/// to determine whether the roll succeeds.
pub trait TargetNumber: Copy + Into<i32> {}

/// The process for determining the result of a test
/// based on its [TargetNumber] and the test roll.
pub trait TestResult<Test>
where
    Test: D20Test,
{
    fn result_for(target: &Test::Target, roll_result: &d20::ValTree) -> Test::Result;
}

/// Allows for ad-hoc decision-making for when the [TargetNumber]
/// for a [D20Test] is not known ahead of time.
///
/// This requires a [Decision] to be made by the GM.
pub trait Disambiguation<Test>: Copy
where
    Test: D20Test,
{
    type Decision: IntoDecision<Response = Test::Result>;
    fn disambiguate(
        test: Weak<Test>,
        creature: Weak<Creature>,
        roll_result: d20::ValTree,
    ) -> Self::Decision;
}

// Shared impls for {Check, Save}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct DC(pub i32);

#[derive(Debug, Clone, Copy)]
pub struct AmbiguousDC;

impl TargetNumber for DC {}

impl From<DC> for i32 {
    fn from(value: DC) -> Self {
        value.0
    }
}

// Trivial implementation of Disambiguation (for [AttackRoll])

impl<Test> Disambiguation<Test> for !
where
    Test: D20Test,
    Test::Result: Response,
{
    type Decision = NoDecision<Test::Result>;

    fn disambiguate(_: Weak<Test>, _: Weak<Creature>, _: d20::ValTree) -> Self::Decision {
        unreachable!()
    }
}

// Nah, didn't ask...

pub struct NoDecision<Response>(PhantomData<Response>);

impl<Response> std::fmt::Debug for NoDecision<Response> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NoDecision").field(&self.0).finish()
    }
}

identity!(@<Response> NoDecision<Response>: Decision, "NO_DECISION");
impl<Response> IntoDecision for NoDecision<Response>
where
    Response: xander_runtime::flow::decision::Response,
{
    type Response = Response;
    type Kind = !;

    fn into_decision(self) -> xander_runtime::flow::io::prelude::Decision {
        unreachable!()
    }
}
