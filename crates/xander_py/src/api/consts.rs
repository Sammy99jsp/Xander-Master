use pyo3::pymodule;

#[pymodule]
pub mod consts {
    #[pymodule_export]
    const SUPPORTED_NON_ATTACK_ACTIONS: usize =
        xander::engine::game::combat::action::SUPPORTED_NON_ATTACK_ACTIONS.len();
    #[pymodule_export]
    const SUPPORTED_MOVEMENT_DIRECTIONS: usize =
        xander::engine::game::combat::turn::DIRECTIONS.len();
    #[pymodule_export]
    const DIRECTION_ARROW: &[&str] = &xander::engine::game::combat::turn::DIRECTION_ARROW;
    #[pymodule_export]
    const FEET_PER_SQUARE: u32 = xander::engine::game::measure::FEET_PER_SQUARE;
}
