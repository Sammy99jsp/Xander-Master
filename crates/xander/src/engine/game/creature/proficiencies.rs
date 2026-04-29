use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

use dynx::Identity;
use xander_runtime::{dynx::cells::InnerValue, lived::list::LivedList};

use crate::prelude::proficiency::{Proficiency, ProficiencyApplicationBase, ProficiencyBase};

type Table = HashMap<String, LivedList<Rc<dyn ProficiencyBase>>>;

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Proficiencies {
    #[rkyv(with = InnerValue<Table>)]
    profs: RefCell<Table>,
}

impl Proficiencies {
    pub fn new() -> Self {
        Self {
            profs: RefCell::new(HashMap::new()),
        }
    }

    pub fn insert<P>(&self, prof: P)
    where
        P: Proficiency,
    {
        let mut profs = self.profs.borrow_mut();
        profs
            .entry(<P::Application as Identity>::LOCAL_ID.to_string())
            .or_default()
            .get_mut()
            .push(Rc::new(prof));
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
