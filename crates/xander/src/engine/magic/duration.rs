use crate::engine::measure;

#[derive(Debug, Clone, Copy)]
pub enum Duration {
    Instantaneous,
    Concentration { up_to: measure::Duration },
    Timed(measure::Duration),
}
