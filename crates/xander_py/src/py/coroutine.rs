use pyo3::{
    Bound, FromPyObject, IntoPyObject, Py, PyAny, PyErr, PyResult, Python, intern,
    types::PyAnyMethods,
};

#[repr(transparent)]
pub struct Coroutine<'py>(Bound<'py, PyAny>);

impl<'py> Coroutine<'py> {
    pub fn new(any: Bound<'py, PyAny>) -> PyResult<Self> {
        // Start the coroutine by passing None to reach the first
        // 'yield' point.
        let py = any.py();

        // Any errors probably imply that this is not a generator/coroutine
        any.call_method1(intern!(py, "send"), (py.None(),))?;

        Ok(Self(any))
    }

    pub fn send<U, T>(&self, value: T) -> PyResult<U>
    where
        T: IntoPyObject<'py>,
        U: for<'a> FromPyObject<'a, 'py>,
    {
        let py = self.0.py();
        let ret = self.0.call_method1(intern!(py, "send"), (value,))?;
        ret.extract::<U>().map_err(Into::<PyErr>::into)
    }

    pub fn unbind(self) -> StoredCoroutine {
        StoredCoroutine(self.0.unbind())
    }
}

#[derive(Debug)]
pub struct StoredCoroutine(Py<PyAny>);

impl StoredCoroutine {
    pub fn clone_ref<'py>(&self, py: Python<'py>) -> Self {
        Self(self.0.clone_ref(py))
    }

    pub fn bind<'py>(&self, py: Python<'py>) -> Coroutine<'py> {
        Coroutine(self.0.clone_ref(py).into_bound(py))
    }
}
