use crate::registry::Registry;

pub trait CustomMetadata {
    const ID: &'static str;

    fn intern(registry: &mut Registry);
}
