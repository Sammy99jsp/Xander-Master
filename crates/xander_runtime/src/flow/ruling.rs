//! Certain [Decision]s can be resolved according to some simple logic.
//! [Ruling]s are conventions that resolve a [Decision] automatically.
//!
//! [IntoRuling] is a helper trait that preserves strong typing,
//! before being converted into its type-erased form, [Ruling], when
//! being registered.
//!
//!
//! Each ruling is tracked by its [Ruling::id] and target decision ([Ruling::on])
//! within the [crate::runtime::Dispatcher].

use downcast_rs::Downcast;

use crate::{
    dynx::{Id, Identity, IntoNamespace, Namespace},
    flow::decision::{Actor, Decision, IntoDecision, Response},
};

/// ## Remember to `register!` your implementation!
pub trait IntoRuling: Identity<Parent = Ruling> + Sized {
    type Decision: IntoDecision;

    /// Rule on a decision.
    fn rule(
        actors: Vec<Actor>,
        kind: <Self::Decision as IntoDecision>::Kind,
    ) -> <Self::Decision as IntoDecision>::Response;

    /// Should this ruling apply to this instance of the decision?
    ///
    /// Defaults to always rule.
    fn applies_to(_actors: &[Actor], _kind: &<Self::Decision as IntoDecision>::Kind) -> bool {
        true
    }

    // In most cases, you do not need to override this function.
    #[inline]
    #[doc(hidden)]
    fn into_ruling() -> Ruling {
        Ruling {
            id: Id::id_for::<Self>(),
            on: Id::id_for::<Self::Decision>(),
            applies_to: |Decision { actors, kind, .. }| {
                Self::applies_to(
                    actors.as_slice(),
                    kind.as_any()
                        .downcast_ref::<<Self::Decision as IntoDecision>::Kind>()
                        .expect("same decision type"),
                )
            },
            rule: |Decision { actors, kind, .. }| {
                let res = Self::rule(
                    actors,
                    *kind
                        .into_any()
                        .downcast::<<Self::Decision as IntoDecision>::Kind>()
                        .unwrap(),
                );

                Box::new(res)
            },
        }
    }
}

#[derive(Clone)]
pub struct Ruling {
    pub id: Id<Self>,
    pub on: Id<Decision>,
    pub applies_to: fn(&Decision) -> bool,
    pub rule: fn(Decision) -> Box<dyn Response>,
}

impl std::fmt::Debug for Ruling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ruling")
            .field("id", &self.id)
            .field("on", &self.on)
            .finish()
    }
}

impl Namespace for Ruling {
    const ID: &'static str = "RULING";
}

impl IntoNamespace for Ruling {
    type Namespace = Self;
}

pub mod prelude {
    pub use super::super::decision::prelude::*;
    pub use super::{IntoRuling, Ruling};
}
