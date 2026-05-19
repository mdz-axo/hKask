//! Bitemporal triples storage

pub struct TripleStore;

impl TripleStore {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TripleStore {
    fn default() -> Self {
        Self::new()
    }
}
