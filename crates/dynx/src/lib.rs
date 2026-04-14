#![feature(unsize)]

pub mod dynx;
pub mod macros;
pub mod registry;

pub use bytecheck;
pub use rkyv;

pub use registry::{Identity, IdentityBase, IntoNamespace, Namespace};

/// Automatically generates implementations for [trait@IntoNamespace] and [trait@Namespace]
///
/// This macro will implement all the necessary traits, types to make your trait a namespace.
///
/// Additionally, you can derive implementations for some [rkyv] traits using the
/// [`derive(..)`](crate::macros::derive) helper.
///
/// # Usage
/// ## Namespace Types
/// Each trait must have a [trait@Namespace] type associated with it.
///
/// By using the
/// ```
/// # use scratchy::Namespace;
/// #
/// #[Namespace("$ID" @ NS)]
/// # pub trait MyNamespace {}
/// ```
/// syntax, the macro will make a [trait@Namespace]-implementing type called `NS` (a unit struct) for you,
/// which will have a [const@Namespace::ID] of `"$ID"`.
///
/// ```
/// use scratchy::Namespace;
///
/// #[Namespace("ID" @ NS, derive(Archive, Serialize, Deserialize))]
/// pub trait MyNamespace {}
/// ```
///
/// Alternatively, you can create your own namespace and reference it by omitting the string literal:
/// ```
/// use scratchy::Namespace;
///
/// #[Namespace(@NS, derive(Archive, Serialize, Deserialize))]
/// pub trait MyNamespace {}
///
/// pub struct NS;
///
/// impl Namespace for NS {
///     const ID: &str = "MY_NAMESPACE";
/// }
/// ```
/// ## Deriving traits
/// See [`derive(..)`](crate::macros::derive) for more information for deriving specific traits.
#[doc(inline)]
pub use dynx_macros::Namespace;

/// Automatically generates [Identity] for an implementation of [macro@Namespace] trait, and optionally
/// registers [rkyv] trait implementations with the global registry.
///
/// # Usage
/// ```
/// # use scratchy::Namespace;
/// use bytecheck::CheckBytes;
/// use rkyv::{Archive, Archived, Deserialize, Serialize};
///
/// use scratchy::Member;
/// #
/// # #[Namespace("ID" @ NS, derive(Archive(ArchivedMyNamespace), Serialize, Deserialize, CheckBytes))]
/// # pub trait MyNamespace {
/// #   fn foo(&self);
/// # }
/// #
/// // Let's assume we already have a namespace trait called `MyNamespace`
/// // with an archive trait called `ArchivedMyNamespace`.
///
/// #[derive(Archive, Serialize, Deserialize, CheckBytes)]
/// pub struct MyImplType;
///
/// #[Member("MY_IMPL_TYPE", register(Archive, Deserialize))]
/// impl MyNamespace for MyImplType {
///     fn foo(&self) {
///         println!("Hello world from MyImplType!")
///     }
/// }
///
/// impl ArchivedMyNamespace for Archived<MyImplType> {}
/// ```
#[doc(inline)]
pub use dynx_macros::Member;
