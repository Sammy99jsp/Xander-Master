use xander_runtime::{DynWeak, Lived, register};

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct TempHp {
    amount: u32,
    source: DynWeak<dyn Lived>,
}

impl Lived for TempHp {
    fn is_alive(&self) -> bool {
        xander_runtime::lived::Lived::is_alive(&self.source) && self.amount > 0
    }
}

register!(TempHp, register(Lived("TEMP_HP")));

// impl xander_runtime::lived::ArchivedLived for ::rkyv::Archived<TempHp> {}

// impl xander_runtime::lived::LivedIdentity for TempHp {
//     fn full_id(&self) -> xander_runtime::dynx::FullId {
//         xander_runtime::dynx::FullId::mononym("TEMP_HP")
//     }
// }

// unsafe impl xander_runtime::dynx::registry::Registered<xander_runtime::lived::Living> for TempHp {}
// unsafe impl xander_runtime::dynx::registry::Registered<Deserializing> for TempHp {}

// ::inventory::submit! {
//     xander_runtime::lived::Living::new::<TempHp>(xander_runtime::dynx::FullId::mononym("TEMP_HP"))
// }

impl TempHp {
    pub fn discount(&mut self, amount: u32) -> u32 {
        let left = amount.saturating_sub(self.amount);
        self.amount = self.amount.saturating_sub(amount);
        left
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, rc::Rc};

    use rkyv::{from_bytes, rancor::Error, to_bytes};
    use xander_runtime::{DynWeak, Lived, always_alive, lived::cell::LivedCell, register};

    use crate::engine::game::health::temp_hp::TempHp;

    #[test]
    fn test_serialize() {
        #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
        pub struct Forever;

        always_alive!(Forever);
        register!(Forever, register(Lived("FOREVER")));

        let forever = Rc::new(Forever) as Rc<dyn Lived>;

        let cell = LivedCell::new(TempHp {
            amount: 32,
            source: DynWeak::new(Rc::downgrade(&forever)),
        });

        let bytes = to_bytes::<Error>(&cell).unwrap();
        fs::write("test.dump", &bytes).unwrap();

        let _ = from_bytes::<LivedCell<TempHp>, Error>(&bytes).unwrap();
    }
}
