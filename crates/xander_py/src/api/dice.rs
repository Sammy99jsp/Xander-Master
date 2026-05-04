use pyo3::prelude::*;

use crate::py::utils::UnsafePythonEscape;

mod rs {
    pub use xander::d20;
}

#[pyclass]
pub struct DExpr(pub UnsafePythonEscape<rs::d20::DExpr>);

#[pymethods]
impl DExpr {
    #[pyo3(name = "__repr__")]
    pub fn repr(&self) -> String {
        self.0.show()
    }
}

#[pyclass]
pub struct ValTree(pub UnsafePythonEscape<rs::d20::ValTree>);

#[pymethods]
impl ValTree {
    #[pyo3(name = "__repr__")]
    pub fn repr(&self) -> String {
        self.0.show()
    }

    pub fn total(&self) -> i32 {
        self.0.total()
    }

    #[pyo3(name = "__int__")]
    pub fn to_int(&self) -> i32 {
        self.0.total()
    }
}
