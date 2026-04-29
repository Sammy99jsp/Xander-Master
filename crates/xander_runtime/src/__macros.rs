#[doc(hidden)]
#[allow(non_snake_case)]
pub mod __register__autocomplete {
    use ::dynx::macros::DocumentationOnly;
    pub type AutocompleteFn = fn(DocumentationOnly, DocumentationOnly);

    pub use ::dynx::{
        macros::register,
        registry::{
            Archiving as Archive, Deserializing as Deserialize, StoredSingleton as Singleton,
        },
    };

    #[allow(non_upper_case_globals)]
    pub fn Lived(_id: &'static str) {}
}
