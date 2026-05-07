use std::marker::PhantomData;

use dynx::{
    Namespace,
    dynx::{Single as InnerSingle, Singleton},
    registry::REGISTRY,
};
use serde::Deserializer;

pub struct WithId<T, Id> {
    pub value: T,
    pub id: Id,
}

#[derive(serde::Deserialize)]
pub struct Single<T>(#[serde(deserialize_with = "deserialize_single")] pub InnerSingle<T>)
where
    T: Singleton + ?Sized + 'static;

impl<T> Clone for Single<T>
where
    T: Singleton + ?Sized + 'static,
{
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

pub fn deserialize_single<'de, D, T>(deserializer: D) -> Result<InnerSingle<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Singleton + ?Sized,
{
    struct Visitor<T>(PhantomData<T>)
    where
        T: Singleton + ?Sized;

    impl<'de, T> serde::de::Visitor<'de> for Visitor<T>
    where
        T: Singleton + ?Sized + 'static,
    {
        type Value = InnerSingle<T>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                f,
                "a reference to a {} singleton (string)",
                <T::Namespace as Namespace>::ID
            )
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            REGISTRY
                .lookup::<T>(v)
                .ok_or_else(|| E::invalid_value(serde::de::Unexpected::Str(v), &self))
                // SAFETY: A stored singleton in the registry is of type T
                .map(|meta| unsafe { meta.stored_singleton.unwrap().cast::<T>() })
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            str::from_utf8(v)
                .map_err(|_| E::invalid_value(serde::de::Unexpected::Bytes(v), &self))
                .and_then(|v| self.visit_str(v))
        }
    }

    deserializer.deserialize_str(Visitor(PhantomData))
}
