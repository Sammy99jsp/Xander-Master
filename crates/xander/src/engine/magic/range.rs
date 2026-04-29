use crate::engine::measure::Feet;

#[derive(Debug, Clone, Copy)]
pub enum Range {
    #[doc(alias = "Feet")]
    Distance(Feet),
    Touch,

    /// `Self` is a reserved keyword in Rust, so Range::Self => Range::Me
    #[doc(alias = "Self")]
    Me,
}
