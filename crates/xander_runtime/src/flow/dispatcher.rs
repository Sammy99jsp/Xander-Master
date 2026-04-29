use std::{
    marker::PhantomData,
    pin::Pin,
    rc::{Rc, Weak},
    task::{ContextBuilder, LocalWake, LocalWaker, Poll, Waker},
};

#[derive(Debug)]
pub struct Dispatcher<State>(Weak<State>)
where
    State: ?Sized;

impl<State> Dispatcher<State>
where
    State: ?Sized,
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
    State: ?Sized,
{
    #[pin]
    pub(crate) fut: Fut,
    pub(crate) dispatcher: Rc<Dispatcher<State>>,
}

pub struct DispatchWaker<State>
where
    State: ?Sized,
{
    dispatcher: Rc<Dispatcher<State>>,
    waker: Waker,
}

impl<State> LocalWake for DispatchWaker<State>
where
    State: ?Sized,
{
    fn wake(self: Rc<Self>) {
        self.waker.wake_by_ref();
    }
}

impl<State, Fut> Future for Dispatched<State, Fut>
where
    Fut: Future,
    State: ?Sized + 'static,
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
    State: ?Sized,
{
    type Output = &'a State;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let waker = cx.local_waker().data() as *const DispatchWaker<State>;

        // SAFETY: Assuming we are wrapped by Dispatch<...> somewhere above,
        //         *waker is always initialized, and waker != null
        //         as it is from an Rc<DispatchWaker>.
        let waker = unsafe { waker.as_ref().unwrap_unchecked() };

        Poll::Ready(waker.dispatcher.state())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hello_world() {
        use super::*;

        let state = <Rc<str> as From<&str>>::from("Hello, world!");
        type MyDispatcher = Dispatcher<str>;
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
