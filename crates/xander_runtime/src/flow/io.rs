pub use crate::flow::decision::{Actor, Decision, IntoDecision};

pub trait Interface: std::fmt::Debug {
    type ActorState;
    fn log(&self, displ: &dyn std::fmt::Display) -> impl IntoFuture<Output = ()>;
    fn prompt<D>(&self, decision: Decision) -> impl IntoFuture<Output = D::Response>
    where
        D: IntoDecision;

    fn decide<D>(&self, decision: D) -> impl IntoFuture<Output = D::Response>
    where
        D: IntoDecision,
    {
        self.prompt::<D>(decision.into_decision())
    }
    fn state_for(&self, actor: Actor) -> &Self::ActorState;
}

pub mod prelude {
    pub use super::super::decision::*;
    pub use super::Interface;
    pub use smol::future::Boxed;
    pub use std::any::Any;
}
