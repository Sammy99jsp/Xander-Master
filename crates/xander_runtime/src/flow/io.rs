use std::rc::Rc;

use futures::future::LocalBoxFuture;

use crate::flow::decision::Response;
pub use crate::flow::decision::{Actor, Decision, IntoDecision};

pub trait Interface: std::fmt::Debug {
    type ActorState: ?Sized;
    fn log<'a, 'b: 'a>(&'a self, displ: &'b dyn std::fmt::Display) -> LocalBoxFuture<'a, ()>;
    fn prompt_dyn<'a>(&'a self, decision: Decision) -> LocalBoxFuture<'a, Box<dyn Response>>;

    fn state_for(&self, actor: Actor) -> Rc<Self::ActorState>;
}

pub trait InterfaceExt: Interface {
    fn decide<D>(&self, decision: D) -> impl IntoFuture<Output = D::Response>
    where
        D: IntoDecision,
    {
        async {
            *self
                .prompt_dyn(decision.into_decision())
                .await
                .downcast::<D::Response>()
                .expect("Downcast to work!")
        }
    }
}

impl<T: Interface + ?Sized> InterfaceExt for T {}

pub mod prelude {
    pub use super::super::decision::*;
    pub use super::Interface;
    pub use smol::future::Boxed;
    pub use std::any::Any;
}

#[derive(Debug)]
pub struct TestInterface;

impl Interface for TestInterface {
    type ActorState = ();

    fn log<'a, 'b: 'a>(&'a self, displ: &'b dyn std::fmt::Display) -> LocalBoxFuture<'a, ()> {
        use futures::FutureExt;

        println!("{displ}");
        std::future::ready(()).boxed_local()
    }

    fn prompt_dyn<'a>(&'a self, decision: Decision) -> LocalBoxFuture<'a, Box<dyn Response>> {
        use futures::FutureExt;

        println!("{decision:?}");

        async {
            panic!("Decisions cannot be decided in test environments.");
        }
        .boxed_local()
    }

    fn state_for(&self, _: Actor) -> Rc<Self::ActorState> {
        Rc::new(())
    }
}
