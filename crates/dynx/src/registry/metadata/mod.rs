pub use super::{Record, Registry};
pub use crate::{Identity, IntoNamespace, Namespace, rkyv::ptr_meta};
pub use std::marker::Unsize;
use std::{any::Any, collections::BTreeMap};

pub mod archiving;
pub mod custom;
pub mod deserializing;
pub mod singleton;

#[derive(Debug, Default)]
pub struct Meta {
    pub archiving: Option<archiving::Archiving>,
    pub deserializing: Option<deserializing::Deserializing>,
    pub stored_singleton: Option<singleton::StoredSingleton>,
    pub extra: BTreeMap<&'static str, Box<dyn Any + Send + Sync>>,
}

pub trait Metadata: Copy {
    fn inscribe(record: Record<Self>, meta: &mut Meta);

    #[allow(unused_variables)]
    fn after_inscribe(record: Record<Self>, registry: &mut Registry) {}
}

/// In most cases, DO NOT implement this trait yourself! This will be auto-impl'd
/// by the [macro@crate::Member] macro.
///
/// # Safety
/// - For [Deserializing]: the [rkyv::Deserialize] implementation for this type must be present in the [Record] in the global [Registry].
/// - For [Archiving]: the [rkyv::traits::ArchivePointee] implementation for this type must be present in the [Record] in the global [Registry].
#[diagnostic::on_unimplemented(
    message = "{T} for {Self} is not recorded in the global Registry.",
    note = "Use the `#[Member(.., register(Archive, Deserialize))]` macro to register this implementation."
)]
pub unsafe trait Registered<T> {}
