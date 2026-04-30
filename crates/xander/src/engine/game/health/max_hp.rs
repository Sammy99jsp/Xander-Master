use xander_runtime::lived::{ArchivedProvisoBase, Provided, Proviso};

#[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct MaxHp {
    inner: Provided<super::HpValue>,
}

pub struct HitPointMaxZero;

impl MaxHp {
    pub const fn new() -> MaxHp {
        MaxHp {
            inner: Provided::new(),
        }
    }

    pub fn with_set(max_hp: u32) -> Option<MaxHp> {
        if max_hp == 0 {
            return None;
        }

        Some(MaxHp {
            inner: {
                let mut prov = Provided::new();
                prov.enroll_mut(provisos::Set(max_hp));
                prov
            },
        })
    }

    pub async fn enroll<P>(&self, part: P) -> Result<(), HitPointMaxZero>
    where
        P: Proviso<u32> + rkyv::Archive + 'static,
        P::Archived: ArchivedProvisoBase<u32>,
    {
        self.inner.enroll(part);

        match self.inner.get().await {
            0 => Err(HitPointMaxZero),
            1.. => Ok(()),
        }
    }

    pub async fn get(&self) -> super::HpValue {
        self.inner.get().await
    }
}

impl Default for MaxHp {
    fn default() -> Self {
        Self::new()
    }
}

pub mod provisos {
    use std::future::ready;

    use xander_runtime::lived::provided::prelude::*;

    #[derive(Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
    pub struct Set(pub(super) u32);

    register!(Set: dyn ProvisoBase<u32>, register(Identity("SET_MAX_HP"), Archive, Deserialize, Lived(always)));

    impl ArchivedProvisoBase<u32> for rkyv::Archived<Set> {}

    impl Proviso<u32> for Set {
        fn provide(&self, t: &mut u32) -> impl IntoFuture<Output = std::ops::ControlFlow<()>> {
            *t = self.0;
            ready(ControlFlow::Continue(()))
        }
    }
}
