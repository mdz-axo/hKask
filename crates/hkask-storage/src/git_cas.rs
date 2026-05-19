//! Git CAS integration

pub struct GitCas;

impl GitCas {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GitCas {
    fn default() -> Self {
        Self::new()
    }
}
