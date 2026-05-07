#[derive(Debug, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum Affiliation {
    /// Solo. No Teams, just 1(v1)^n
    #[default]
    None,
}

impl Affiliation {
    pub fn is_friendly(&self, _: &Self) -> bool {
        false
    }
}
