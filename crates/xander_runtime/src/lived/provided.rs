//! A [Provided] value is derived from multiple [Proviso]s.
//!
//! A [Proviso] is a (potentially temporary) effect that modifies a provided value.
//! A provided value starts off from some base value (typically [Default::default]),
//! and each [Proviso] (in ascending priority order) is applied to that value.

use std::{
    marker::PhantomData,
    ops::{ControlFlow, Deref},
    pin::Pin,
    rc::Rc,
};

use ::dynx::{
    dynx::DynCheckBytes,
    registry::{
        Archiving, Deserializing, REGISTRY, Registered, metadata::archiving::ArchivedLocalId,
    },
};
use bytecheck::CheckBytes;
use rkyv::{
    Archive, ArchiveUnsized, DeserializeUnsized, SerializeUnsized, ptr_meta,
    rancor::Fallible,
    ser::{Allocator, Sharing, Writer},
    traits::ArchivePointee,
};

use crate::{
    dynx::{self, Identity},
    lived::{Lived, list::LivedList},
};

pub trait Proviso<T>:
    ProvisoBase<T> + Identity<Parent = dyn ProvisoBase<T>> + Archive + Sized
where
    T: ?Sized + 'static,
    Self: Archive,
    Self::Archived: ArchivedProvisoBase<T>
        + dynx::registry::Registered<Archiving>
        + dynx::registry::Registered<Deserializing>,
{
    const PRIORITY: usize = usize::MAX;

    #[must_use]
    fn provide(&self, t: &mut T) -> impl IntoFuture<Output = ControlFlow<()>>;
}

impl<T, P> ProvisoBase<T> for P
where
    T: ?Sized + 'static,
    P: Proviso<T> + Identity<Parent = dyn ProvisoBase<T>>,
    P::Archived: ArchivedProvisoBase<T>,
{
    fn provide<'t, 's: 't>(
        &'s self,
        t: &'t mut T,
    ) -> Pin<Box<dyn Future<Output = ControlFlow<()>> + 't>> {
        Box::pin(<Self as Proviso<T>>::provide(self, t).into_future())
    }
}

pub type Cause = ();

pub trait ProvisoBase<T>:
    Lived + dynx::IdentityBase<NS<T>> + std::fmt::Debug + for<'a> dynx::dynx::DynSerializeUnsized<'a>
where
    T: ?Sized + 'static,
{
    fn priority(&self) -> usize {
        usize::MAX
    }

    #[must_use]
    fn provide<'t, 's: 't>(
        &'s self,
        x: &'t mut T,
    ) -> Pin<Box<dyn Future<Output = ControlFlow<()>> + 't>>;
}

impl<T> dynx::IntoNamespace for dyn ProvisoBase<T>
where
    T: ?Sized + 'static,
{
    type Namespace = NS<T>;
}

pub struct NS<T>(PhantomData<T>)
where
    T: ?Sized;

impl<T> dynx::Namespace for NS<T>
where
    T: ?Sized + 'static,
{
    const ID: &'static str = "PROVISO";
}

#[repr(transparent)]
#[derive(Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Provided<T>(LivedList<Rc<dyn ProvisoBase<T>>>)
where
    T: 'static;

impl<T> Provided<T>
where
    T: 'static,
{
    pub const fn new() -> Self {
        Self(LivedList::new())
    }

    pub fn enroll_mut<P>(&mut self, proviso: P)
    where
        P: Proviso<T> + 'static,
        P::Archived: ArchivedProvisoBase<T>,
    {
        self.enroll_erased_mut(Rc::new(proviso));
    }

    pub fn enroll<P>(&self, proviso: P)
    where
        P: Proviso<T> + 'static,
        P::Archived: ArchivedProvisoBase<T>,
    {
        self.enroll_erased(Rc::new(proviso))
    }

    pub fn enroll_erased(&self, proviso: Rc<dyn ProvisoBase<T>>) {
        let mut contents = self.0.write();
        contents.push(proviso);
        contents.sort_by_key(|prov: &Rc<dyn ProvisoBase<T> + 'static>| prov.priority())
    }

    pub fn enroll_erased_mut(&mut self, proviso: Rc<dyn ProvisoBase<T>>) {
        let contents = self.0.get_mut();
        contents.push(proviso);
        contents.sort_by_key(|prov| prov.priority());
    }

    pub fn contains<P>(&self) -> bool
    where
        P: Proviso<T>,
        P::Archived: ArchivedProvisoBase<T>,
    {
        self.0
            .try_read()
            .expect("Should not have writers")
            .iter()
            .any(|p| p.local_id() == P::LOCAL_ID)
    }

    #[inline]
    pub async fn get(&self) -> T
    where
        T: Default,
    {
        self.provide(T::default()).await
    }

    pub async fn provide(&self, starting_value: T) -> T {
        let mut t = starting_value;
        // NOTE: This does not account for provisos being added halfway through...
        let provisos = self.0.read().clone();

        for f in provisos {
            match f.provide(&mut t).await {
                ControlFlow::Continue(()) => continue,
                ControlFlow::Break(()) => break,
            }
        }

        t
    }
}

impl<T> Default for Provided<T>
where
    T: 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> std::fmt::Debug for Provided<T>
where
    T: 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(provisos) = self.0.try_read() {
            f.write_str("Provided[")?;
            for (i, proviso) in provisos.iter().map(Rc::deref).enumerate() {
                proviso.fmt(f)?;

                if i < (provisos.len() - 1) {
                    f.write_str(">")?;
                }
            }

            f.write_str("]")
        } else {
            f.write_str("Provided(<Unavailable>)")
        }
    }
}

pub mod prelude {
    pub use super::{ArchivedProvisoBase, Cause, Provided, Proviso, ProvisoBase};
    pub use crate::lived::Lived;
    pub use crate::register;
    pub use std::{
        ops::ControlFlow,
        rc::{Rc, Weak},
    };
}

#[diagnostic::on_unimplemented(
    label = "You have not registered this proviso as archivable and deserializable with the global registry.",
    note = "Please use the register macro: `register!(MyProvisoNameHere: dyn ProvisoBase<{T}>);` "
)]
pub trait ArchivedProvisoBase<T: ?Sized>:
    rkyv::Portable + Registered<Archiving> + Registered<Deserializing> + for<'a> DynCheckBytes<'a>
{
}

impl<T> ArchivePointee for dyn ArchivedProvisoBase<T> + '_
where
    T: ?Sized + 'static,
{
    type ArchivedMetadata = ArchivedLocalId;

    fn pointer_metadata(
        archived: &Self::ArchivedMetadata,
    ) -> <Self as ptr_meta::Pointee>::Metadata {
        unsafe {
            REGISTRY
                .lookup_by_local::<dyn ProvisoBase<T>>(*archived)
                .unwrap()
                .archiving
                .unwrap()
                .cast::<dyn ProvisoBase<T>>()
                .archived
        }
    }
}

impl<'a, T> ArchiveUnsized for dyn ProvisoBase<T> + 'a
where
    T: ?Sized + 'static,
{
    type Archived = dyn ArchivedProvisoBase<T> + 'a;

    fn archived_metadata(&self) -> rkyv::ArchivedMetadata<Self> {
        ArchivedLocalId::new(self.local_id())
    }
}

impl<S, T> SerializeUnsized<S> for dyn ProvisoBase<T> + '_
where
    T: ?Sized + 'static,
    S: Fallible + Writer + Sharing + Allocator + ?Sized,
    S::Error: core::error::Error + Send + Sync + 'static,
{
    fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, <S as Fallible>::Error> {
        unsafe { dynx::dynx::utils::serialize::serialize_unsized(self, serializer) }
    }
}

unsafe impl<T> ptr_meta::Pointee for dyn ProvisoBase<T> + '_
where
    T: ?Sized + 'static,
{
    type Metadata = ptr_meta::DynMetadata<Self>;
}

unsafe impl<T> ptr_meta::Pointee for dyn ArchivedProvisoBase<T> + '_
where
    T: ?Sized + 'static,
{
    type Metadata = ptr_meta::DynMetadata<Self>;
}

impl<T, D> DeserializeUnsized<dyn ProvisoBase<T>, D> for dyn ArchivedProvisoBase<T>
where
    T: ?Sized + 'static,
    D: Fallible + ?Sized,
    D::Error: 'static,
{
    unsafe fn deserialize_unsized(
        &self,
        deserializer: &mut D,
        out: *mut dyn ProvisoBase<T>,
    ) -> Result<(), <D as Fallible>::Error> {
        unsafe { dynx::dynx::utils::deserialize::deserialize_unsized(self, deserializer, out) }
    }

    fn deserialize_metadata(&self) -> <dyn ProvisoBase<T> as ptr_meta::Pointee>::Metadata {
        dynx::dynx::utils::deserialize::deserialize_metadata::<dyn ProvisoBase<T>>(self)
    }
}

unsafe impl<T, C> CheckBytes<C> for dyn ArchivedProvisoBase<T> + '_
where
    T: ?Sized + 'static,
    C: Fallible + ?Sized,
    C::Error: core::error::Error + Send + Sync + 'static,
{
    unsafe fn check_bytes(
        value: *const Self,
        context: &mut C,
    ) -> Result<(), <C as Fallible>::Error> {
        unsafe { dynx::dynx::utils::check_bytes::check_bytes(value, context) }
    }
}

impl<T: 'static> rkyv::traits::LayoutRaw for dyn ProvisoBase<T> + '_ {
    fn layout_raw(
        metadata: <Self as ptr_meta::Pointee>::Metadata,
    ) -> Result<std::alloc::Layout, std::alloc::LayoutError> {
        Ok(metadata.layout())
    }
}

impl<T: 'static> rkyv::traits::LayoutRaw for dyn ArchivedProvisoBase<T> + '_ {
    fn layout_raw(
        metadata: <Self as ptr_meta::Pointee>::Metadata,
    ) -> Result<std::alloc::Layout, std::alloc::LayoutError> {
        Ok(metadata.layout())
    }
}

#[cfg(test)]
mod tests {
    use std::{future::ready, ops::ControlFlow};

    use dynx::Identity;
    use rkyv::{Archive, Deserialize, Serialize, from_bytes, rancor::Error, to_bytes};

    use crate::{
        lived::provided::{ArchivedProvisoBase, Provided, Proviso, ProvisoBase},
        register,
    };

    #[test]
    fn test() {
        #[derive(Debug, Archive, Serialize, Deserialize)]
        pub struct BaseAC(u32);

        register!(BaseAC: dyn ProvisoBase<u32>, register(Archive, Deserialize, Lived(always)));
        // register!(BaseAC: dyn ProvisoBase<u32>, register(Archive(a), Deserialize(a)));
        // register!(BaseAC: dyn ProvisoBase<u32>, register(Archive, Deserialize, Lived));
        impl ArchivedProvisoBase<u32> for ArchivedBaseAC {}

        impl Identity for BaseAC {
            type Parent = dyn ProvisoBase<u32>;
            const LOCAL_ID: &'static str = "BASE_AC";
        }

        // register_proviso!(BaseAC as Proviso<u32>);
        impl Proviso<u32> for BaseAC {
            const PRIORITY: usize = 0;
            fn provide(&self, t: &mut u32) -> impl IntoFuture<Output = ControlFlow<()>> {
                *t = self.0;

                ready(ControlFlow::Continue(()))
            }
        }

        let mut ac = Provided::new();
        ac.enroll_mut(BaseAC(10));
        let bytes = to_bytes::<Error>(&ac).unwrap();

        let _ = from_bytes::<Provided<u32>, Error>(&bytes).unwrap();
    }
}
