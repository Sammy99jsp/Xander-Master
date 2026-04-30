use dynx::Namespace;
use xander_runtime::Lived;

#[Namespace("MAGIC::AREA_OF_EFFECT" @ NS, derive(Archive, Serialize, Deserialize))]
pub trait AreaOfEffect: Lived + std::fmt::Debug {}
