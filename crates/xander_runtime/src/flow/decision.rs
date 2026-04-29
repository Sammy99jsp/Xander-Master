use downcast_rs::{Downcast, impl_downcast};

use crate::{
    dynx::{Id, Identity, IntoNamespace, Namespace},
    ui,
};

pub trait IntoDecision: Identity<Parent = Decision> + Downcast + Sized {
    type Response: Response;
    type Kind: DecisionKind;

    /// Converts this Decision into a type-erased [Decision].
    /// Use [Decision::new<Self>] to achieve this.
    fn into_decision(self) -> Decision;
}

pub trait DecisionKind: Downcast + std::fmt::Debug {
    fn is_multi(&self) -> bool {
        false
    }
}
impl_downcast!(DecisionKind);

pub trait Response: Downcast + std::fmt::Debug + 'static {}
impl_downcast!(Response);

impl<R> Response for R where R: Downcast + std::fmt::Debug + 'static {}

#[derive(Debug)]
pub struct Decision {
    id: Id<Self>,
    pub actors: Vec<Actor>,
    pub component: ui::Component,
    pub kind: Box<dyn DecisionKind>,
}

impl Namespace for Decision {
    const ID: &'static str = "DECISION";
}

/// [Decision] itself is the namespace for all [IntoDecision] types.
impl IntoNamespace for Decision {
    type Namespace = Self;
}

impl Decision {
    pub fn new<D>(actors: Vec<Actor>, component: ui::Component, kind: D::Kind) -> Self
    where
        D: IntoDecision,
    {
        Self {
            id: Id::id_for::<D>(),
            actors,
            component,
            kind: Box::new(kind),
        }
    }
}

/// Some external being that interacts with the
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Actor(u32);

impl std::fmt::Debug for Actor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Actor")
            .field_with(|f| match self.0 {
                0 => f.write_str("GM"),
                id @ 1.. => write!(f, "{id}"),
            })
            .finish()
    }
}

impl Actor {
    const GM_ID: u32 = 0;
    pub const GM: Self = Self(Self::GM_ID);

    pub const fn is_gm(&self) -> bool {
        self.0 == Self::GM_ID
    }

    pub const fn as_index(&self) -> usize {
        self.0 as usize
    }
}

impl Decision {
    pub const fn id(&self) -> &Id<Self> {
        &self.id
    }
}

type ValidateFn = dyn for<'a> Fn(&'a [Box<dyn ui::UI>]) -> Result<(), ui::Component> + Send + Sync;

pub struct Selection {
    pub items: Vec<Box<dyn ui::UI>>,
    pub validate: Option<Box<ValidateFn>>,
    pub qty: usize,
}

impl DecisionKind for Selection {}

impl std::fmt::Debug for Selection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Selection")
            .field("items", &self.items)
            .field("qty", &self.qty)
            .finish()
    }
}

pub struct Ranking {
    pub items: Vec<Box<dyn ui::UI>>,
    pub validate: Option<Box<ValidateFn>>,
}

impl DecisionKind for Ranking {}

impl std::fmt::Debug for Ranking {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Selection")
            .field("items", &self.items)
            .finish()
    }
}

impl DecisionKind for Vec<Box<dyn DecisionKind>> {
    fn is_multi(&self) -> bool {
        true
    }
}

impl DecisionKind for ! {
    fn is_multi(&self) -> bool {
        unreachable!()
    }
}

pub mod prelude {
    pub use super::{Actor, Decision, DecisionKind, IntoDecision, Ranking, Selection};
    pub use crate::{
        dynx::Identity,
        ui::{Component, UI},
    };
    pub use crate::{identity, register};
}
