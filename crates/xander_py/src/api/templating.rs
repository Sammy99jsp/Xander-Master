use pyo3::pymodule;

#[pymodule]
pub mod templating {
    use std::{ops::Deref, rc::Rc};

    use pyo3::{
        exceptions::{PyIOError, PyValueError},
        prelude::*,
        types::PyTuple,
    };
    use xander::engine::json::{self, serde_json};

    use crate::{
        api,
        py::utils::{MaybeStrong, PyFile, PythonOwnedRc, PythonWeak, UnsafePythonEscape},
    };

    fn load<'py, T, U>(
        args: &Bound<'py, PyTuple>,
        map: impl FnOnce(T) -> PyResult<U>,
    ) -> PyResult<U>
    where
        T: xander::serde::de::DeserializeOwned,
    {
        if args.len() == 0 {
            return Err(PyValueError::new_err(
                "Expected either a file-like or file path",
            ));
        }

        let file = PyFile::from_str_or_file(&args.get_item(0)?, false)?;
        let raw = serde_json::from_reader::<_, T>(file.0)
            .map_err(|err| PyIOError::new_err(err.to_string()))?;

        map(raw)
    }

    #[pyclass]
    pub struct Creature(UnsafePythonEscape<json::creature::Creature>);

    #[pymethods]
    impl Creature {
        #[staticmethod]
        #[pyo3(signature = (*args))]
        pub fn load_json<'py>(args: &Bound<'py, PyTuple>) -> PyResult<Self> {
            load(args, |raw: json::creature::Creature| unsafe {
                Ok(Self(UnsafePythonEscape::new(raw)))
            })
        }

        #[pyo3(signature = (game, *, name = None))]
        pub fn make<'py>(
            &self,
            game: PyRef<'py, api::game::Game>,
            name: Option<String>,
        ) -> PyResult<api::game::Creature> {
            let mut raw = self.0.deref().clone();

            if let Some(name) = name {
                raw.name = name;
            }

            let game = PythonOwnedRc::into_inner(game.game.clone());
            let weak = Rc::downgrade(&game);
            let creature = game.combat.load_raw_creature(raw);

            Ok(api::game::Creature {
                creature: unsafe { MaybeStrong::strong(creature) },
                game: unsafe { PythonWeak::new(weak) },
            })
        }
    }
}
