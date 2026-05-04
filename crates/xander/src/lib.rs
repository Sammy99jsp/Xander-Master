#![allow(internal_features)]
#![feature(
    rustc_attrs,
    int_roundings,
    debug_closure_helpers,
    never_type,
    box_patterns,
    unwrap_infallible,
    ptr_metadata,
)]

pub mod engine;
pub mod prelude;
pub mod utils;
pub mod content;

pub use d20;
pub use xander_runtime as runtime;
