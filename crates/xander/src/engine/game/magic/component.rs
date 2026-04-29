use xander_runtime::ui;

#[derive(Debug, Clone)]
pub enum Component {
    Verbal,
    Somatic,
    Material(Box<dyn MaterialBase>),
}

impl ui::Ui for Component {}

/// Because material components behave differently,
/// this is a trait instead.
///
/// Common functionality (i.e. one/multiple ingredients with no value)
/// can be found in helper types that implement Material.
pub trait Material: Clone + MaterialBase {}

impl<M: Material> MaterialBase for M {
    fn cloned(&self) -> Box<dyn MaterialBase> {
        Box::new(self.clone())
    }
}

pub trait MaterialBase: ui::Ui + Send + Sync {
    fn cloned(&self) -> Box<dyn MaterialBase>;
}

impl Clone for Box<dyn MaterialBase> {
    fn clone(&self) -> Self {
        self.cloned()
    }
}

// Helpers for components![..] macro.

/// [Component::Verbal]
#[doc(hidden)]
pub const V: Component = Component::Verbal;

/// [Component::Somatic]
#[doc(hidden)]
pub const S: Component = Component::Somatic;

#[doc(hidden)]
#[allow(non_snake_case)]
/// [Component::Material] for some [Material] `M`.
pub fn M<M: Material>(material: M) -> Component {
    Component::Material(Box::new(material))
}
