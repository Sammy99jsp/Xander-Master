use std::{cell::Cell, num::NonZero};

use xander_runtime::{DynWeak, Lived, dynx::cells::InnerValue, lived::LivedSerializable, register};

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct TempHp {
    #[rkyv(with = InnerValue<super::HpValue>)]
    hp_left: Cell<super::HpValue>,
    source: DynWeak<dyn LivedSerializable>,
}

impl Lived for TempHp {
    fn is_alive(&self) -> bool {
        xander_runtime::lived::Lived::is_alive(&self.source) && self.hp_left.get() > 0
    }
}

register!(TempHp, register(Identity("TEMP_HP"), Lived(@)));

pub struct Discounted {
    pub discounted: NonZero<super::HpValue>,
    pub remaining: super::HpValue,
}

impl TempHp {
    pub fn discount(&self, damage: u32) -> Discounted {
        let discounted = self.hp_left.get().min(damage);
        let remaining = damage.saturating_sub(self.hp_left.get());

        // Update internal count.
        self.hp_left.update(|left| left.saturating_sub(damage));

        debug_assert!(discounted > 0);

        Discounted {
            // SAFETY: if we are even alive, then discounted >= 1 (as otherwise, we'd be not alive...)
            //         This assumption is covered by the debug_assert!(..)
            discounted: unsafe { NonZero::try_from(discounted).unwrap_unchecked() },
            remaining,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, fs, rc::Rc};

    use rkyv::{from_bytes, rancor::Error, to_bytes};
    use xander_runtime::{
        DynWeak,
        lived::{LivedSerializable, cell::LivedCell},
        register,
    };

    use crate::engine::game::health::temp_hp::TempHp;

    #[test]
    fn test_serialize() {
        #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
        pub struct Forever;

        register!(Forever, register(Identity("TEST::FOREVER"), Lived(@always)));

        let forever = Rc::new(Forever) as Rc<dyn LivedSerializable>;

        let cell = LivedCell::new(TempHp {
            hp_left: Cell::new(32),
            source: DynWeak::new(Rc::downgrade(&forever)),
        });

        let bytes = to_bytes::<Error>(&cell).unwrap();
        fs::write("test.dump", &bytes).unwrap();

        let _ = from_bytes::<LivedCell<TempHp>, Error>(&bytes).unwrap();
    }
}
