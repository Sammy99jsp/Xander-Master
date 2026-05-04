use pyo3::{PyClass, exceptions::PyValueError, prelude::*};

mod rs {
    pub use xander::engine::game::combat::utils::Availability;
}

#[pyclass]
pub struct Illegal(String);

impl Illegal {
    pub fn new<E: std::fmt::Display>(value: E) -> Self {
        Self(value.to_string())
    }
}

#[pymethods]
impl Illegal {
    #[pyo3(name = "__repr__")]
    pub fn repr(&self) -> String {
        format!("Illegal({})", &self.0)
    }
}

pub fn expired(msg: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(format!("{msg} expired"))
}

#[pyclass]
pub struct Availability {
    inner: rs::Availability<Py<PyAny>>,
}

impl Availability {
    pub fn new<T>(py: Python<'_>, availability: rs::Availability<T>) -> PyResult<Self>
    where
        T: PyClass + Into<PyClassInitializer<T>>,
    {
        let is_available = availability.is_available();
        let value: Py<T> = Py::new(py, availability.value())?;
        Ok(Self {
            inner: match is_available {
                true => rs::Availability::available(value.into_any()),
                false => rs::Availability::unavailable(value.into_any()),
            },
        })
    }

    pub fn from_any(any: rs::Availability<Py<PyAny>>) -> Self {
        Self { inner: any }
    }
}

#[pymethods]
impl Availability {
    pub fn is_available(&self) -> bool {
        self.inner.is_available()
    }

    pub fn value<'py>(&self, py: Python<'py>) -> &Bound<'py, PyAny> {
        self.inner.as_ref().value().bind(py)
    }

    #[pyo3(name = "__repr__")]
    pub fn repr(&self, py: Python<'_>) -> PyResult<String> {
        let inner = self
            .inner
            .as_ref()
            .value()
            .call_method0(py, "__repr__")?
            .extract::<String>(py)?;

        Ok(match self.is_available() {
            true => format!("Available({inner})"),
            false => format!("Unavailable({inner})"),
        })
    }
}
