//! Cascade composition

pub struct Cascade;

impl Cascade {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Cascade {
    fn default() -> Self {
        Self::new()
    }
}
