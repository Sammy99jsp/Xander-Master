// These imports are actually used for hover/autocomplete information within macros.
#![allow(unused_imports)]

use super::{Member, Namespace, dynx::Single, dynx::Singleton};
use rkyv::{ArchiveUnsized, DeserializeUnsized, SerializeUnsized, bytecheck::CheckBytes};

#[doc(hidden)]
pub enum DocumentationOnly {}

// Little hack for the #[Namespace(..)] macro. This function below is for the `derive()` helper.
// By binding the span of the `derive` to the function below, the user will see the docs on hover.

/// The `derive` helper.
///
/// ```
/// # use dynx::Namespace;
/// # #[Namespace("TR" @ NS,
/// derive(Archive, Serialize, Deserialize, CheckBytes)
/// # )]
/// # pub trait Tr {}
/// ```
///
/// This will help implement all of the necessary traits for your unsized
/// type. `derive(..)` supports the following traits:
/// - `Archive` => [ArchiveUnsized]
/// - `Serialize` => [SerializeUnsized]
/// - `Deserialize` => [DeserializeUnsized]
/// - `CheckBytes` => [CheckBytes]
/// - `Singleton` => [Singleton]
///
/// # Usage
/// ## Singleton Traits
/// If your trait is only implemented by singleton types (aka unit structs),
/// you may use `derive(Singleton)`.
/// ```
/// # use dynx::Namespace;
/// # #[Namespace("TR" @ NS,
/// derive(Singleton)
/// # )]
/// # pub trait Tr {}
/// ```
///
/// Singleton namespace traits can be freely archived, serialized, and deserialized
/// through [Single] without any extra work.
///
/// ## Archiving
///
/// When you write `derive(Archive, ..)`, the [macro@Namespace] macro will generate a special "archive trait"
/// (e.g. `TrArchive` for your trait called `Tr`), which will have all of the necessary
/// supertrait requirements for `dyn Tr` and `dyn TrArchive` to implement the respective traits
/// (e.g. [SerializeUnsized], [DeserializeUnsized], etc.).
///
/// ## Explicit Archive Trait Name
/// You can explicitly specify the name for the archive trait by using:
/// ```
/// # use dynx::Namespace;
/// # #[Namespace("TR" @ NS,
/// derive(Archive(MyArchiveTraitNameHere))
/// # )]
/// # pub trait Tr {}
/// ```
///
/// ## Existing Archive Trait
/// Alternatively, you can tell `derive` to use an already existing trait by using an `@`:
/// ```ignore
/// derive(Archive(@MyArchiveTrait))
/// ```
///
/// This allows you to write your own interface for the archived version of
/// your trait object:
///
/// ```
/// use dynx::{
///     dynx::DynCheckBytes, Namespace,
///     registry::{Registered, Archiving, Deserializing}
/// };
///
/// #[Namespace("TR" @ NS, derive(Archive(@ArchivedTr), /* Serialize, Deserialize, CheckBytes */))]
/// pub trait Tr {
///     fn name(&self) -> &str;
/// }
///
/// // Note that you will need to manually add supertrait requirements
/// // to your archived trait:
/// pub trait ArchivedTr:
///     rkyv::Portable              // <-- Always required
///     + Registered<Archiving>     // <-- Always required
///     + Registered<Deserializing> // <-- for `derive(Deserialize, ..)`
///     + for<'a> DynCheckBytes<'a> // <-- for `derive(CheckBytes, ..)`  
/// {}
/// ```
///
pub fn derive(_: DocumentationOnly) {}

/// The `register` helper.
///
/// ```
/// # use bytecheck::{CheckBytes};
/// # use rkyv::{Archive, Deserialize, Serialize};
/// # use dynx::{Namespace, Member};
/// #
/// # #[Namespace("TR" @ NS, derive(Archive, Serialize, Deserialize, CheckBytes))]
/// # pub trait NamespaceTr {}
/// #
/// # #[derive(Archive, Serialize, Deserialize, CheckBytes)]
/// # pub struct ImplA;
/// #
/// # #[Member("IMPL_A",
/// register(Archive, Deserialize)
/// # )]
/// # impl NamespaceTr for ImplA {}
/// #
/// # impl ArchivedNamespaceTr for ArchivedImplA {}
/// ```
///
/// This will help register all of the various implementations of the following traits
/// with the global registry:
/// - `Archive` => [ArchiveUnsized]
/// - `Deserialize` => [DeserializeUnsized]
/// - `Singleton` => [Singleton]
///
/// If you do not need to use register these implementations, you can entirely omit [`register(..)`](register) from
/// the [`#[Member(..)]`](macro@Member) macro.
///
#[allow(unused_variables, non_snake_case)]
pub fn register(Archive: DocumentationOnly, Deserialize: DocumentationOnly) {}

