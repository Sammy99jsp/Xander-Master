use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use dynx::Identity;
use rkyv::{
    bytecheck,
    de::Pooling,
    rancor::{Fallible, Source},
};
use xander_runtime::lived::list::LivedList;

use crate::prelude::proficiency::{Proficiency, ProficiencyApplicationBase, ProficiencyBase};

type Table = HashMap<String, LivedList<Rc<dyn ProficiencyBase>>>;

pub struct Proficiencies {
    profs: RefCell<Table>,
}

impl Proficiencies {
    pub fn new() -> Self {
        Self {
            profs: RefCell::new(HashMap::new()),
        }
    }

    pub fn insert_mut<P>(&mut self, prof: P)
    where
        P: Proficiency,
    {
        let profs = self.profs.get_mut();
        profs
            .entry(<P::Application as Identity>::LOCAL_ID.to_string())
            .or_default()
            .get_mut()
            .push(Rc::new(prof));
    }

    pub async fn get(
        &self,
        app: &dyn ProficiencyApplicationBase,
    ) -> Option<Weak<dyn ProficiencyBase>> {
        let profs = self.profs.borrow();
        let r = profs.get(app.local_id())?;

        r.read()
            .iter()
            .find(|prof| prof.applies_to(app))
            .map(Rc::downgrade)
    }
}

impl Default for Proficiencies {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Proficiencies {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(profs) = self.profs.try_borrow() {
            f.write_str("Proficiencies ")?;
            profs.fmt(f)
        } else {
            f.debug_tuple("Proficiencies")
                .field_with(|f| f.write_str("<Unavailable>"))
                .finish()
        }
    }
}

#[derive(rkyv::Archive)]
pub struct Test(String);

// Archiving
#[repr(transparent)]
#[derive(rkyv::Portable, bytecheck::CheckBytes)]
#[bytecheck(crate = rkyv::bytecheck)]
pub struct ArchivedTable(rkyv::Archived<Table>);

impl ArchivedTable {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.0.values().map(|a| a.len()).sum()
    }

    pub fn on(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(rkyv::Archived::<String>::as_str)
    }
}

pub struct ResolverTable(rkyv::Resolver<Table>);

impl rkyv::Archive for Proficiencies {
    type Archived = ArchivedTable;
    type Resolver = ResolverTable;

    fn resolve(&self, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
        let table_ptr = unsafe { ::core::ptr::addr_of_mut!((*out.ptr()).0) };
        let table_out = unsafe { ::rkyv::Place::from_field_unchecked(out, table_ptr) };
        self.profs.borrow().resolve(resolver.0, table_out);
    }
}

impl<S> rkyv::Serialize<S> for Proficiencies
where
    S: Fallible + ?Sized,
    Table: rkyv::Serialize<S>,
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, <S as Fallible>::Error> {
        let resolver = self.profs.borrow().serialize(serializer)?;
        Ok(ResolverTable(resolver))
    }
}

impl<D> rkyv::Deserialize<Proficiencies, D> for rkyv::Archived<Proficiencies>
where
    D: Fallible + Pooling + ?Sized,
    D::Error: Source,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<Proficiencies, <D as Fallible>::Error> {
        let table = self.0.deserialize(deserializer)?;
        Ok(Proficiencies {
            profs: RefCell::new(table),
        })
    }
}
