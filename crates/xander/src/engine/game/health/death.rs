use xander_runtime::ui;

#[derive(Debug, Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Dead;

impl ui::Ui for Dead {}