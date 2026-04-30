use std::{fmt::Debug, rc::Rc};

use downcast_rs::{Downcast, impl_downcast};
use dynx::{Identity, Namespace};
use xander_runtime::{DynWeak, dynx::Id, lived::Lived};

use crate::engine::game::creature::Creature;

pub trait Proficiency: Identity<Parent = dyn ProficiencyBase> + ProficiencyBase {
    type Application: ProficiencyApplication;
    fn applies_to(&self, app: &Self::Application) -> bool;
}

impl<Prof> ProficiencyBase for Prof
where
    Prof: Proficiency,
{
    fn application_id(&self) -> &'static Id<dyn ProficiencyApplicationBase> {
        const { &Id::id_for::<Prof::Application>() }
    }

    fn applies_to(&self, app: &dyn ProficiencyApplicationBase) -> bool {
        let Some(app) = app
            .as_any()
            .downcast_ref::<<Self as Proficiency>::Application>()
        else {
            return false;
        };

        Proficiency::applies_to(self, app)
    }
}

#[Namespace("PROFICIENCY" @ NS, derive(Archive, Serialize, Deserialize, CheckBytes))]
pub trait ProficiencyBase: Lived + Downcast + Debug {
    fn application_id(&self) -> &'static Id<dyn ProficiencyApplicationBase>;
    fn applies_to(&self, app: &dyn ProficiencyApplicationBase) -> bool;
}

impl_downcast!(ProficiencyBase);

pub trait ProficiencyApplication:
    Identity<Parent = dyn ProficiencyApplicationBase> + ProficiencyApplicationBase + Clone
{
}

impl<App> ProficiencyApplicationBase for App
where
    App: ProficiencyApplication,
{
    fn boxed_clone(&self) -> Box<dyn ProficiencyApplicationBase> {
        Box::new(Clone::clone(self))
    }
}

#[Namespace("PROFICIENCY_APPLICATION" @ AppNS, derive(Archive, Serialize, Deserialize))]
pub trait ProficiencyApplicationBase: Downcast + Debug {
    fn boxed_clone(&self) -> Box<dyn ProficiencyApplicationBase>;
}

impl_downcast!(ProficiencyApplicationBase);

impl dyn ProficiencyApplicationBase + '_ {
    /// Checks if this value itself is `P`, or if it is
    /// a combinatorial with includes `P`.
    pub fn contains<P>(&self, app: &P) -> bool
    where
        P: ProficiencyApplication + PartialEq,
    {
        // TODO: Check for combinatorics (like OR) when they are implemented.
        self.downcast_ref::<P>() == Some(app)
    }
}

impl Clone for Box<dyn ProficiencyApplicationBase> {
    fn clone(&self) -> Self {
        ProficiencyApplicationBase::boxed_clone(self.as_ref())
    }
}

#[repr(transparent)]
#[rustc_layout_scalar_valid_range_start(2)] // (+2)
#[rustc_layout_scalar_valid_range_end(9)] // (+9)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProficiencyBonus(pub(crate) i8);

// TODO: Introduce a new-type for d20::DExpr that mandates the
//          "The Bonus Doesn't Stack" section.

impl Default for ProficiencyBonus {
    fn default() -> Self {
        unsafe { Self(0) }
    }
}

impl ProficiencyBonus {
    pub const fn value(&self) -> i32 {
        self.0 as i32
    }

    pub fn into_expr(
        &self,
        me: DynWeak<Creature>,
        prof: DynWeak<dyn ProficiencyBase>,
    ) -> d20::DExpr {
        d20::DExpr::from(self.value()).label(Rc::new(ui::ProficiencyBonusHint { me, prof }))
    }
}

pub mod ui {

    use xander_runtime::{DynWeak, register};

    use crate::engine::game::{creature::Creature, stats::proficiency::ProficiencyBase};

    #[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct ProficiencyBonusHint {
        pub me: DynWeak<Creature>,
        pub prof: DynWeak<dyn ProficiencyBase>,
    }

    impl xander_runtime::ui::Ui for ProficiencyBonusHint {}
    register!(
        ProficiencyBonusHint,
        register(Identity("PROFICIENCY_BONUS_HINT"))
    );
}
