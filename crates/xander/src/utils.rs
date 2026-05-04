use dynx::{Identity, IdentityBase, IntoNamespace, Namespace};

pub trait IdentityExt<NS: Namespace>: IdentityBase<NS> {
    fn is<U>(&self) -> bool
    where
        U: Identity,
        U::Parent: IntoNamespace<Namespace = NS>;

    fn downcast<U>(&self) -> Option<&U>
    where
        U: Identity,
        U::Parent: IntoNamespace<Namespace = NS>,
    {
        if !self.is::<U>() {
            return None;
        }

        let (data, _meta) = (self as *const Self).to_raw_parts();

        // SAFETY: Local ID's must be necessarily unique!
        unsafe { (data as *const U).as_ref() }
    }
}

impl<NS, T> IdentityExt<NS> for T
where
    NS: Namespace,
    T: IdentityBase<NS> + ?Sized,
{
    fn is<U>(&self) -> bool
    where
        U: Identity,
        U::Parent: IntoNamespace<Namespace = NS>,
    {
        self.local_id() == U::LOCAL_ID
    }
}
