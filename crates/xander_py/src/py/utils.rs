use std::{
    ops::Deref,
    rc::{Rc, Weak as RcWeak},
    sync::{Arc, Weak as ArcWeak},
};

mod rs {
    pub use xander::engine::game::Game;
}

use pyo3::{PyResult, exceptions::PyValueError};
use xander::runtime::smol;

pub struct PythonOwnedRc<T: ?Sized>(Rc<T>);

impl<T: ?Sized> PythonOwnedRc<T> {
    pub fn strong_count(this: &Self) -> usize {
        Rc::strong_count(&this.0)
    }

    pub fn weak_count(this: &Self) -> usize {
        Rc::weak_count(&this.0)
    }

    pub fn into_inner(this: Self) -> Rc<T> {
        this.0
    }

    pub fn downgrade(this: &Self) -> RcWeak<T> {
        Rc::downgrade(&this.0)
    }
}

impl<T: ?Sized> Deref for PythonOwnedRc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct PythonWeak<T: ?Sized>(RcWeak<T>);

impl<T: ?Sized> Clone for PythonWeak<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: ?Sized> PythonWeak<T> {
    pub unsafe fn new(weak: RcWeak<T>) -> Self {
        Self(weak)
    }
}

impl<T: ?Sized> PythonOwnedRc<T> {
    pub unsafe fn new(rc: Rc<T>) -> Self {
        Self(rc)
    }
}

pub struct UnsafePythonEscape<T>(T);

impl<T> UnsafePythonEscape<T> {
    pub const unsafe fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T> Deref for UnsafePythonEscape<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// TODO: This is probably a very bad idea.
//       We should find a way to protect this via
//       some clever semaphore or something...
unsafe impl<T> Send for PythonWeak<T> {}
unsafe impl<T> Sync for PythonWeak<T> {}

unsafe impl<T> Send for PythonOwnedRc<T> {}
unsafe impl<T> Sync for PythonOwnedRc<T> {}

unsafe impl<T> Send for UnsafePythonEscape<T> {}
unsafe impl<T> Sync for UnsafePythonEscape<T> {}

pub trait OrExpired<T: ?Sized> {
    type Pointer<U: ?Sized>: Deref<Target = U>;
    fn upgrade_or_expired(&self, msg: impl std::fmt::Display) -> PyResult<Self::Pointer<T>>;
}

impl<T: ?Sized> OrExpired<T> for PythonWeak<T> {
    type Pointer<U: ?Sized> = Rc<U>;

    fn upgrade_or_expired(&self, msg: impl std::fmt::Display) -> PyResult<Self::Pointer<T>> {
        self.0
            .upgrade()
            .ok_or_else(|| PyValueError::new_err(format!("{msg} expired")))
    }
}

impl<T: ?Sized> OrExpired<T> for RcWeak<T> {
    type Pointer<U: ?Sized> = Rc<U>;

    fn upgrade_or_expired(&self, msg: impl std::fmt::Display) -> PyResult<Self::Pointer<T>> {
        self.upgrade()
            .ok_or_else(|| PyValueError::new_err(format!("{msg} expired")))
    }
}

impl<T: ?Sized> OrExpired<T> for ArcWeak<T> {
    type Pointer<U: ?Sized> = Arc<U>;

    fn upgrade_or_expired(&self, msg: impl std::fmt::Display) -> PyResult<Self::Pointer<T>> {
        self.upgrade()
            .ok_or_else(|| PyValueError::new_err(format!("{msg} expired")))
    }
}

pub fn run_future<T>(game: Rc<rs::Game>, future: impl IntoFuture<Output = T>) -> T {
    smol::block_on(game.dispatcher.dispatch(future.into_future()))
}
