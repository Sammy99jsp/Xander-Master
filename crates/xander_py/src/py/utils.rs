use std::{
    ops::Deref,
    rc::{Rc, Weak as RcWeak},
    sync::{Arc, Weak as ArcWeak},
};

mod rs {
    pub use xander::engine::game::Game;
}

use pyo3::{
    PyResult,
    exceptions::{PyIOError, PyValueError},
};
use xander::runtime::smol;

pub struct PythonOwnedRc<T: ?Sized>(Rc<T>);

impl<T: ?Sized> Clone for PythonOwnedRc<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

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

impl<T: ?Sized> PythonWeak<T> {
    pub fn as_inner(&self) -> RcWeak<T> {
        self.0.clone()
    }
}

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
    pub unsafe fn new_cyclic(f: impl for<'a> FnOnce(&'a RcWeak<T>) -> T) -> Self
    where
        T: Sized,
    {
        Self(Rc::new_cyclic(f))
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

#[derive(Debug)]
pub enum MaybeStrong<T: ?Sized> {
    Strong(Rc<T>),
    Weak(RcWeak<T>),
}

impl<T: ?Sized> MaybeStrong<T> {
    pub unsafe fn strong(rc: Rc<T>) -> Self {
        Self::Strong(rc)
    }
}

impl<T: ?Sized> MaybeStrong<T> {
    pub fn take_strong(&mut self) -> Option<Rc<T>> {
        match self {
            MaybeStrong::Strong(s) => {
                let a = s.clone();
                *self = MaybeStrong::Weak(Rc::downgrade(&a));
                Some(a)
            }
            MaybeStrong::Weak(_) => None,
        }
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

unsafe impl<T> Send for MaybeStrong<T> {}
unsafe impl<T> Sync for MaybeStrong<T> {}

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

impl<T: ?Sized> OrExpired<T> for MaybeStrong<T> {
    type Pointer<U: ?Sized> = Rc<U>;

    fn upgrade_or_expired(&self, msg: impl std::fmt::Display) -> PyResult<Self::Pointer<T>> {
        match self {
            MaybeStrong::Strong(s) => Ok(s.clone()),
            MaybeStrong::Weak(weak) => weak
                .upgrade()
                .ok_or_else(|| PyValueError::new_err(format!("{msg} expired"))),
        }
    }
}

pub fn run_future<T>(game: Rc<rs::Game>, future: impl IntoFuture<Output = T>) -> T {
    smol::block_on(game.dispatcher.dispatch(future.into_future()))
}

use std::os::fd::{FromRawFd, RawFd};

use pyo3::{exceptions::PyTypeError, intern, prelude::*, sync::PyOnceLock};
pub struct PyFile(pub std::fs::File);

impl PyFile {
    pub fn from_str_or_file<'py>(any: &'py Bound<'py, PyAny>, write: bool) -> PyResult<Self> {
        match any {
            arg if let Ok(p) = arg.extract::<String>() => std::fs::OpenOptions::new()
                .read(!write)
                .write(write)
                .truncate(write)
                .open(&p)
                .map_err(|err| PyIOError::new_err(err.to_string()))
                .map(Self),
            arg if let Ok(file @ PyFile(_)) = PyFile::from_io_base(arg) => Ok(file),
            _ => Err(PyValueError::new_err(
                "schema functions only accepts nothing, a file path, or a TextIOWrapper",
            )),
        }
    }

    pub fn from_io_base<'py>(any: &'py Bound<'py, PyAny>) -> PyResult<Self> {
        let py = any.py();

        static IO_BASE: PyOnceLock<Py<PyAny>> = PyOnceLock::new();

        let io_base = IO_BASE.get_or_init(py, || {
            py.import("io").unwrap().getattr("IOBase").unwrap().unbind()
        });

        let io_base = io_base.bind(py);

        if !any.is_instance(io_base)? {
            return Err(PyTypeError::new_err(
                "Expected a BaseIO instance, such as open(..)",
            ));
        }

        let fileno = any
            .call_method0(intern!(py, "fileno"))?
            .extract::<RawFd>()?;

        // SAFETY: this is valid from Python.
        let file = unsafe { std::fs::File::from_raw_fd(fileno) };

        Ok(Self(file))
    }
}
