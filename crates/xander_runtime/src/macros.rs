#![allow(non_snake_case)]

#[doc(hidden)]
pub type AutocompleteFn = fn();

/// Register macro helper
pub fn register() {}

#[doc(inline)]
pub use crate::lived::register_lived as Lived;
pub fn Lived() {}

#[doc(inline)]
pub use crate::dynx::de::register_deserialize as Deserialize;
pub fn Deserialize() {}

#[doc(inline)]
pub use crate::dynx::archiving::register_archive as Archive;
pub fn Archive() {}

#[doc(inline)]
pub use crate::dynx::register_identity as Identity;
#[expect(unused_variables)]
pub fn Identity(local_id: &str) {}
