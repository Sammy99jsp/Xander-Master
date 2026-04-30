pub mod archiving;
pub mod cells;
pub mod de;
pub mod weak;

use std::marker::PhantomData;

pub use ::dynx::*;

pub struct Id<P: ?Sized> {
    local_id: &'static str,
    _namespace: PhantomData<P>,
}

impl<P> Id<P>
where
    P: IntoNamespace + ?Sized,
{
    pub const fn id_for<T>() -> Self
    where
        T: Identity<Parent = P>,
    {
        Self {
            local_id: T::LOCAL_ID,
            _namespace: PhantomData,
        }
    }
}

impl<P> Copy for Id<P> where P: IntoNamespace + ?Sized {}

impl<P> Clone for Id<P>
where
    P: IntoNamespace + ?Sized,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<P> std::fmt::Debug for Id<P>
where
    P: IntoNamespace + ?Sized,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", P::Namespace::ID, self.local_id)
    }
}

#[doc(hidden)]
pub use crate::register_identity;

#[doc(hidden)]
#[macro_export]
macro_rules! register_identity {
    ($(@<$($g: ident),*$(,)?>)? ($id: expr) $this: path: $tr: ty) => {
        impl$(<$($g),*>)? $crate::dynx::Identity for $this {
            type Parent = $tr;
            const LOCAL_ID: &'static str = const { $id };
        }
    };

    ($(@<$($g: ident),*$(,)?>)? ($id: expr) $this: path) => {
        impl$(<$($g),*>)? $crate::dynx::registry::identity::IdentityFull for $this {
            const FULL_ID: $crate::dynx::registry::identity::FullId = $crate::dynx::registry::identity::FullId::mononym($id);
        }
    };
}

#[macro_export]
macro_rules! identity {
    () => {};
}
