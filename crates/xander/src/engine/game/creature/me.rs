use std::{ops::Deref, rc::Weak};

use crate::engine::game::creature::Creature;

/// [Me] is a serializable/deserializable (weak) reference to a creature
/// with a guarantee that there is always at least (one) strong reference
/// to the creature.
///
/// Use this type for things that are self-referential to the creature
/// that holds a value.
///
/// For external creatures, you may use the usual Weak<Creature>.
///
/// This type is primarily to allow for cyclical references within the [rkyv] ecosystem.
///
#[derive(Clone)]
pub struct Me(pub(super) Weak<Creature>);

impl std::fmt::Debug for Me {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Me").finish()
    }
}

impl Deref for Me {
    type Target = Creature;

    fn deref(&self) -> &Self::Target {
        if self.0.strong_count() == 0 {
            panic!("Tried to dereference me without any strong references!");
        }

        unsafe { self.0.as_ptr().as_ref().unwrap() }
    }
}

pub mod archiving {
    use std::{
        mem::MaybeUninit,
        rc::{Rc, Weak},
    };

    use dynx::dynx::DynDeserializer;
    use rkyv::{
        Archive, Deserialize, Serialize,
        de::Pooling,
        rancor::{Fallible, Source},
    };

    use crate::engine::game::creature::{ArchivedCreatureId, Creature, CreatureId};

    use super::Me;

    impl Archive for Me {
        type Archived = rkyv::Archived<CreatureId>;
        type Resolver = rkyv::Resolver<CreatureId>;

        fn resolve(&self, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
            self.id.resolve(resolver, out);
        }
    }

    impl<S> Serialize<S> for Me
    where
        S: Fallible + ?Sized,
    {
        fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, <S as Fallible>::Error> {
            self.id.serialize(serializer)
        }
    }

    impl<D> Deserialize<Me, D> for rkyv::Archived<Me>
    where
        D: Fallible + ?Sized,
    {
        fn deserialize(&self, deserializer: &mut D) -> Result<Me, <D as Fallible>::Error> {
            // SAFETY: We should only be called by something that is owned by Creature...
            let deserializer = unsafe {
                (deserializer as *mut D as *mut Deserializer<'_, dyn DynDeserializer>)
                    .as_mut()
                    .unwrap_unchecked()
            };

            Ok(Me(deserializer.creature(self.to_id())))
        }
    }

    pub trait CreatureDeserializer: Fallible {
        fn creature(&self, id: CreatureId) -> Weak<Creature>;
    }

    pub struct Deserializer<'a, D: ?Sized> {
        creature: Rc<MaybeUninit<Creature>>,
        inner: &'a mut D,
    }

    impl<'a, D> CreatureDeserializer for Deserializer<'a, D>
    where
        D: Fallible + ?Sized,
    {
        fn creature(&self, _: CreatureId) -> Weak<Creature> {
            let weak = Rc::downgrade(&self.creature);

            unsafe { Weak::from_raw(Weak::into_raw(weak).cast::<Creature>()) }
        }
    }

    impl<'a, D> Fallible for Deserializer<'a, D>
    where
        D: Fallible + ?Sized,
    {
        type Error = D::Error;
    }

    impl<'a, D> Pooling for Deserializer<'a, D>
    where
        D: Fallible + Pooling + ?Sized,
    {
        #[inline]
        fn start_pooling(&mut self, address: usize) -> rkyv::de::PoolingState {
            self.inner.start_pooling(address)
        }

        unsafe fn finish_pooling(
            &mut self,
            address: usize,
            ptr: rkyv::de::ErasedPtr,
            drop: unsafe fn(rkyv::de::ErasedPtr),
        ) -> Result<(), <Self as Fallible>::Error> {
            unsafe { self.inner.finish_pooling(address, ptr, drop) }
        }
    }

    impl<D> Deserialize<Rc<Creature>, D> for rkyv::Archived<Creature>
    where
        D: Fallible + Pooling + ?Sized,
        D::Error: Source,
    {
        fn deserialize(
            &self,
            deserializer: &mut D,
        ) -> Result<Rc<Creature>, <D as Fallible>::Error> {
            let mut deserializer = Deserializer {
                creature: Rc::new_uninit(),
                inner: deserializer,
            };

            let ptr = deserializer.creature.as_ptr().cast_mut();

            unsafe {
                std::ptr::write(
                    ptr,
                    Creature {
                        id: self.id.deserialize(&mut deserializer)?,
                        name: self.name.deserialize(&mut deserializer)?,
                        size: self.size.deserialize(&mut deserializer)?,
                        kind: self.kind.deserialize(&mut deserializer)?,
                        stats: self.stats.deserialize(&mut deserializer)?,
                    },
                )
            };

            unsafe { Ok(deserializer.creature.assume_init()) }
        }
    }

    impl ArchivedCreatureId {
        fn to_id(&self) -> CreatureId {
            CreatureId(self.0.to_native())
        }
    }
}
