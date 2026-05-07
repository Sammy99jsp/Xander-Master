use pyo3::{
    IntoPyObjectExt,
    exceptions::{PyIOError, PyValueError},
    prelude::*,
    types::PyTuple,
};

use crate::py::utils::PyFile;

mod rs {
    pub use xander::engine::json;
}

fn write_schema<T>(writer: impl std::io::Write) -> PyResult<()>
where
    T: rs::json::schemars::JsonSchema,
{
    let schema = rs::json::schemars::SchemaGenerator::default().into_root_schema_for::<T>();
    xander::engine::json::serde_json::to_writer_pretty(writer, &schema)
        .map_err(|err| PyIOError::new_err(err.to_string()))
}

fn generate<'py, T>(py: Python<'py>, args: &Bound<'py, PyTuple>) -> PyResult<Bound<'py, PyAny>>
where
    T: rs::json::schemars::JsonSchema,
{
    if args.len() > 1 {
        return Err(PyValueError::new_err(
            "schema functions only accepts nothing, a file path, or a TextIOWrapper",
        ));
    }

    if args.is_empty() {
        // return as string
        let mut s = String::new();
        // SAFETY: The JSON schema will be valid UTF-8
        unsafe { write_schema::<T>(s.as_mut_vec())? };
        return s.into_bound_py_any(py);
    }

    let arg = args.get_item(0)?;

    let file = PyFile::from_str_or_file(&arg, true)?;

    write_schema::<T>(file.0)?;

    Ok(py.None().into_bound(py))
}

#[pymodule]
pub mod schema {
    use pyo3::{prelude::*, types::PyTuple};
    use xander::engine::json;

    use super::generate;

    #[pyfunction]
    #[pyo3(signature = (*args))]
    pub fn creature<'py>(
        py: Python<'py>,
        args: &Bound<'py, PyTuple>,
    ) -> PyResult<Bound<'py, PyAny>> {
        generate::<json::creature::Creature>(py, args)
    }

    #[pyfunction]
    #[pyo3(signature = (*args))]
    pub fn game<'py>(py: Python<'py>, args: &Bound<'py, PyTuple>) -> PyResult<Bound<'py, PyAny>> {
        generate::<json::game::Game>(py, args)
    }
}
