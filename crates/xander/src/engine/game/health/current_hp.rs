use std::cell::Cell;

use super::HpValue;

#[derive(Debug)]
pub struct CurrentHp {
    pub(super) value: Cell<HpValue>,
}

impl CurrentHp {
    pub(super) const fn new(value: HpValue) -> Self {
        Self {
            value: Cell::new(value),
        }
    }
}

pub mod archiving {
    use std::cell::Cell;

    use crate::engine::game::health::HpValue;

    use super::CurrentHp;
    use rkyv::{Archive, Deserialize, Serialize, bytecheck::CheckBytes, rancor::Fallible};

    #[repr(transparent)]
    #[derive(rkyv::Portable, CheckBytes)]
    #[bytecheck(crate = rkyv::bytecheck)]
    pub struct ArchivedCurrentHp(rkyv::Archived<HpValue>);

    impl Archive for CurrentHp {
        type Archived = ArchivedCurrentHp;
        type Resolver = ();

        fn resolve(&self, _: Self::Resolver, out: rkyv::Place<Self::Archived>) {
            rkyv::munge::munge!(let ArchivedCurrentHp(inner) = out);
            inner.write(rkyv::Archived::<HpValue>::from_native(self.value.get()));
        }
    }

    impl<S> Serialize<S> for CurrentHp
    where
        S: Fallible + ?Sized,
    {
        fn serialize(&self, _: &mut S) -> Result<Self::Resolver, <S as Fallible>::Error> {
            Ok(())
        }
    }

    impl<D> Deserialize<CurrentHp, D> for rkyv::Archived<CurrentHp>
    where
        D: Fallible + ?Sized,
    {
        fn deserialize(&self, _: &mut D) -> Result<CurrentHp, <D as Fallible>::Error> {
            Ok(CurrentHp {
                value: Cell::new(self.0.to_native()),
            })
        }
    }
}
