use dynx::{Member, Namespace};
use xander_runtime::ui;

#[Namespace("SPELL::TARGETING" @ NS, derive(Singleton))]
pub trait Targeting: ui::UI + Send + Sync {
    fn cloned(&self) -> Box<dyn Targeting>;
}

impl Clone for Box<dyn Targeting> {
    fn clone(&self) -> Self {
        self.cloned()
    }
}

#[derive(Debug, Clone)]
pub struct Creature;

impl ui::UI for Creature {}

#[Member("CREATURE", register(Singleton))]
impl Targeting for Creature {
    fn cloned(&self) -> Box<dyn Targeting> {
        Box::new(self.clone())
    }
}
