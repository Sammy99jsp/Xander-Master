use std::rc::Weak;

use downcast_rs::{Downcast, impl_downcast};

pub trait Ui: Downcast + std::fmt::Debug {}
impl_downcast!(Ui);

#[derive(Debug)]
pub enum Component {
    Text(String),
    RefRich(Weak<dyn Ui>),
    Rich(Box<dyn Ui>),
    Multi(Vec<Component>),
}

impl FromIterator<Self> for Component {
    fn from_iter<T: IntoIterator<Item = Self>>(iter: T) -> Self {
        Self::Multi(Vec::from_iter(iter))
    }
}
