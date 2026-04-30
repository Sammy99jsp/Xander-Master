use xander_runtime::{register, ui};

#[derive(Debug, Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Dead;
register!(Dead, register(Identity("DEAD")));

impl ui::Ui for Dead {}
