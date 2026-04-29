use xander_runtime::lived::cell::LivedCell;

use crate::engine::game::health::temp_hp::TempHp;

pub mod temp_hp;

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Health {
    pub temp_hp: LivedCell<TempHp>,
}
