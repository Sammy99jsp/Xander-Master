use std::{
    future::ready,
    marker::PhantomData,
    pin::Pin,
    rc::{Rc, Weak},
    task::{ContextBuilder, LocalWake, LocalWaker, Poll, Waker},
};

use crate::flow::{
    self, Event,
    event::{EventHandler, Outcome},
};

pub trait DispatchState {
    type Interface: flow::Interface + ?Sized;
    fn interface(&self) -> &Self::Interface;

    fn handle<E: Event<Self>>(&self, event: E) -> impl IntoFuture<Output = Outcome<Self, E>>;
    fn listen<H>(&self, handler: H)
    where
        H: EventHandler<Self> + 'static;

    fn update(
        &self,
    ) -> impl IntoFuture<Output = Result<(), <Self::Interface as flow::Interface>::IoError>> {
        ready(Ok(()))
    }
}

#[derive(Debug)]
pub struct Dispatcher<State>(Weak<State>)
where
    State: DispatchState + ?Sized;

impl<State> Dispatcher<State>
where
    State: DispatchState + ?Sized,
{
    /// Creates a new dispatcher for the provided state.
    ///
    /// # Safety
    /// The [Rc<State>] counterpart to `weak` must outlive this value.
    ///
    /// This is achievable by calling this function within [Rc::new_cyclic]
    /// for [Rc<State>].
    pub unsafe fn new(weak: Weak<State>) -> Rc<Self> {
        Rc::new(Self(weak))
    }

    #[must_use]
    pub fn dispatch<Fut>(self: &Rc<Self>, fut: Fut) -> Dispatched<State, Fut>
    where
        Fut: Future,
    {
        Dispatched {
            fut,
            dispatcher: self.clone(),
        }
    }

    pub fn local<'b>() -> LocalDispatcher<'b, State> {
        LocalDispatcher(PhantomData)
    }

    pub fn state(&self) -> &State {
        debug_assert!(
            self.0.strong_count() > 0,
            "State has been dropped before its corresponding Dispatcher"
        );

        // SAFETY: As part of the guarantee in [Self::new],
        //         the Rc<State> must outlive the Dispatcher,
        //         So, there will always be at least one strong reference to <State>.
        unsafe { self.0.as_ptr().as_ref().unwrap_unchecked() }
    }
}

#[pin_project::pin_project]
pub struct Dispatched<State, Fut>
where
    State: DispatchState + ?Sized,
{
    #[pin]
    pub(crate) fut: Fut,
    pub(crate) dispatcher: Rc<Dispatcher<State>>,
}

pub struct DispatchWaker<State>
where
    State: DispatchState + ?Sized,
{
    dispatcher: Rc<Dispatcher<State>>,
    waker: Waker,
}

impl<State> LocalWake for DispatchWaker<State>
where
    State: DispatchState + ?Sized,
{
    fn wake(self: Rc<Self>) {
        self.waker.wake_by_ref();
    }
}

impl<State, Fut> Future for Dispatched<State, Fut>
where
    Fut: Future,
    State: DispatchState + ?Sized + 'static,
{
    type Output = Fut::Output;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let local_waker = LocalWaker::from(Rc::new(DispatchWaker {
            dispatcher: self.dispatcher.clone(),
            waker: cx.waker().clone(),
        }));

        let mut cx = ContextBuilder::from_waker(Waker::noop())
            .local_waker(&local_waker)
            .build();

        let fut = self.project().fut;
        fut.poll(&mut cx)
    }
}

pub struct LocalDispatcher<'a, State>(PhantomData<&'a State>)
where
    State: ?Sized;

impl<'a, State> Future for LocalDispatcher<'a, State>
where
    State: DispatchState + ?Sized,
{
    type Output = &'a State;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let waker = cx.local_waker().data() as *const DispatchWaker<State>;

        // SAFETY: Assuming we are wrapped by Dispatch<...> somewhere above,
        //         *waker is always initialized, and waker != null
        //         as it is from an Rc<DispatchWaker>.
        let waker = unsafe { waker.as_ref().unwrap_unchecked() };
        let state: &State = waker.dispatcher.state();

        Poll::Ready(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::io::TestInterface;

    type MyDispatcher = Dispatcher<str>;

    impl DispatchState for str {
        type Interface = TestInterface;

        fn interface(&self) -> &Self::Interface {
            &TestInterface
        }

        fn handle<E: Event<Self>>(&self, event: E) -> impl IntoFuture<Output = Outcome<Self, E>> {
            event.finalize()
        }

        fn listen<H>(&self, _: H)
        where
            H: EventHandler<Self>,
        {
        }
    }

    #[test]
    fn hello_world() {
        let state = <Rc<str> as From<&str>>::from("Hello, world!");

        let dispatcher = Rc::new(Dispatcher(Rc::downgrade(&state)));

        smol::block_on(async move {
            dispatcher
                .dispatch(async {
                    let local = MyDispatcher::local().await;
                    println!("From inside the future: {}", local);
                })
                .await;
        })
    }
}
