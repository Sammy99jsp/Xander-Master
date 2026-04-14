use rkyv::rancor::Fallible;

use crate::dynx::error::DynError;

pub trait DynDeserializer {}
impl<D> DynDeserializer for D where D: Fallible + ?Sized {}

impl<'a> Fallible for dyn DynDeserializer + 'a {
    type Error = DynError;
}
