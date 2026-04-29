use std::rc::Weak;

use downcast_rs::{Downcast, impl_downcast};

pub trait UI: Downcast + std::fmt::Debug {}
impl_downcast!(UI);

#[derive(Debug)]
pub enum Component {
    Text(String),
    RefRich(Weak<dyn UI>),
    Rich(Box<dyn UI>),
    Multi(Vec<Component>),
}

impl FromIterator<Self> for Component {
    fn from_iter<T: IntoIterator<Item = Self>>(iter: T) -> Self {
        Self::Multi(Vec::from_iter(iter))
    }
}
