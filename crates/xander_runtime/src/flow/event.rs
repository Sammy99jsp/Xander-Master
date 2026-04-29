use std::{future::ready, pin::Pin};

use downcast_rs::{Downcast, impl_downcast};
use dynx::{IntoNamespace, Namespace};
use futures::future::LocalBoxFuture;

use crate::{
    dynx::{Identity, IdentityBase},
    flow::{Dispatcher, dispatcher::DispatchState},
    lived::Lived,
};

pub trait Event<S: ?Sized>: Sized + Identity<Parent = dyn EventBase<S>> + EventBase<S> {
    type Resolved;
    fn map_resolved(self) -> impl IntoFuture<Output = Self::Resolved>;

    type Cancelled;
    fn map_cancelled(self) -> impl IntoFuture<Output = Self::Cancelled>;

    fn finalize(self) -> impl IntoFuture<Output = Outcome<S, Self>> {
        async {
            match self.is_cancelled() {
                false => Outcome::Resolved(self.map_resolved().await),
                true => Outcome::Cancelled(self.map_cancelled().await),
            }
        }
    }

    fn handle(self) -> impl IntoFuture<Output = Outcome<S, Self>>
    where
        S: DispatchState,
    {
        async {
            let state = Dispatcher::<S>::local().await;
            state.handle(self).await
        }
    }
}

pub struct NS;

impl Namespace for NS {
    const ID: &'static str = "EVENT";
}

/// The base trait for game events.
///
/// In most cases, do not implement this trait yourself, implement [Event] instead.
///
/// This trait is primarily used for cases where we need to erase the type of event,
/// as [Event] is not dyn-compatible.
#[diagnostic::on_unimplemented(
    message = "EventBase not implemented for {Self}",
    note = "If your Event is cancellable, add an EventState field, and use cancellable!({Self}, self.event_state) to auto-impl EventBase."
)]
pub trait EventBase<S: ?Sized>: IdentityBase<self::NS> + std::fmt::Debug + Downcast {
    fn is_cancelled(&self) -> bool;
}

impl<S: ?Sized> IntoNamespace for dyn EventBase<S> {
    type Namespace = NS;
}

impl_downcast!(EventBase<S>);

/// Implements [EventBase] for an [Event], if `Self` has a named Option<[Event::Cancelled]> field.
///
/// ### Syntax
/// ```ignore
/// cancellable!(MyEvent, cancel_reason_opt_field_name);
/// ```    
///
/// ### Example
/// ```ignore
/// pub use xander_runtime::flow::event::*;
///
/// #[derive(Debug)]
/// pub struct TestEvent {
///     cancel_reason: Option<()>,
/// }
///
/// impl Identity for TestEvent {
///     type Trait = dyn EventBase<State>;
///     const LOCAL_ID: &'static str = "TEST::TEST_EVENT";
/// }
///
/// impl<State> Event<State> for TestEvent {
///     type Resolved = String;
///     fn map_resolved(
///         self,
///     ) -> impl IntoFuture<Output = Self::Resolved> {
///         async { "Resolved!".to_string() }
///     }
///
///     type Cancelled = ();
///     fn map_cancelled(
///         self,
///     ) -> impl IntoFuture<Output = Self::Cancelled> {
///         async { self.cancel_reason.unwrap()  }
///     }
/// }
///
/// // Uses self.state
/// cancellable!(TestEvent, state);
/// ```
#[macro_export]
macro_rules! cancellable {
    ($subject: ty, $field: ident) => {
        impl<D> $crate::flow::event::EventBase<D> for $subject {
            fn is_cancelled(&self) -> bool {
                Option::is_some(&self.$field)
            }
        }
    };
}

pub use crate::cancellable;

/// Blanket implementation for non-cancellable events.
impl<S, E> EventBase<S> for E
where
    E: Event<S, Cancelled = !>,
{
    fn is_cancelled(&self) -> bool {
        false
    }
}

// Event Handlers.

pub trait EventHandler<S: ?Sized>: EventHandlerBase<S> {
    type Event: Event<S>;
    fn handle<'s, 'e: 's>(
        &'s self,
        event: &'e mut Self::Event,
    ) -> impl IntoFuture<Output = ()> + 's;
}

pub trait EventHandlerBase<S: ?Sized>: Lived + std::fmt::Debug {
    fn handle<'s, 'e: 's>(&'s self, event: &'e mut dyn EventBase<S>) -> LocalBoxFuture<'s, ()>;
}

impl<S, H> EventHandlerBase<S> for H
where
    H: EventHandler<S>,
    S: 'static,
{
    fn handle<'s, 'e: 's>(
        &'s self,
        event: &'e mut dyn EventBase<S>,
    ) -> Pin<Box<dyn Future<Output = ()> + 's>> {
        if let Some(event) = event.downcast_mut::<H::Event>() {
            Box::pin(EventHandler::handle(self, event).into_future())
        } else {
            Box::pin(ready(()))
        }
    }
}

/// Result after an event has been handled by the [Dispatcher] and appropriate [EventHandler]s.
pub enum Outcome<S: ?Sized, E>
where
    E: Event<S>,
{
    Resolved(E::Resolved),
    Cancelled(E::Cancelled),
}

impl<S, E> Outcome<S, E>
where
    E: Event<S>,
{
    pub fn is_resolved(&self) -> bool {
        matches!(self, Self::Resolved(_))
    }

    pub fn into_result(self) -> Result<E::Resolved, E::Cancelled> {
        match self {
            Outcome::Resolved(resolved) => Ok(resolved),
            Outcome::Cancelled(err) => Err(err),
        }
    }
}

impl<S, T, E> Outcome<S, E>
where
    E: Event<S, Cancelled = !, Resolved = T>,
{
    /// Immediately get the inner value of this [Outcome]
    /// if the event cannot be cancelled.
    pub fn inner(self) -> T {
        match self {
            Outcome::Resolved(t) => t,
        }
    }
}

impl<S, E> std::fmt::Debug for Outcome<S, E>
where
    E: Event<S>,
    E::Resolved: std::fmt::Debug,
    E::Cancelled: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resolved(arg0) => f.debug_tuple("Resolved").field(arg0).finish(),
            Self::Cancelled(arg0) => f.debug_tuple("Cancelled").field(arg0).finish(),
        }
    }
}

impl<S, E> std::ops::Try for Outcome<S, E>
where
    E: Event<S>,
{
    type Output = E::Resolved;
    type Residual = E::Cancelled;

    fn from_output(output: Self::Output) -> Self {
        Outcome::Resolved(output)
    }

    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        match self {
            Outcome::Resolved(resolved) => std::ops::ControlFlow::Continue(resolved),
            Outcome::Cancelled(cancelled) => std::ops::ControlFlow::Break(cancelled),
        }
    }
}

impl<S, E, Cancelled> std::ops::FromResidual for Outcome<S, E>
where
    E: Event<S, Cancelled = Cancelled>,
{
    fn from_residual(residual: <Self as std::ops::Try>::Residual) -> Self {
        Outcome::Cancelled(residual)
    }
}

#[cfg(test)]
mod tests {
    use std::future::ready;

    use super::{Event, EventBase};
    use crate::dynx::Identity;

    #[test]
    fn event() {
        #[derive(Debug, Default)]
        pub struct TestEvent {
            cancel_reason: Option<()>,
        }

        type State = ();

        impl Identity for TestEvent {
            type Parent = dyn EventBase<State>;
            const LOCAL_ID: &'static str = "TEST::TEST_EVENT";
        }

        impl Event<State> for TestEvent {
            type Resolved = String;
            fn map_resolved(self) -> impl IntoFuture<Output = Self::Resolved> {
                ready("Done!".to_string())
            }

            type Cancelled = ();
            fn map_cancelled(self) -> impl IntoFuture<Output = Self::Cancelled> {
                ready(())
            }
        }

        cancellable!(TestEvent, cancel_reason);

        let evt = TestEvent::default();
        let _ = &evt as &dyn EventBase<State>;
    }
}
