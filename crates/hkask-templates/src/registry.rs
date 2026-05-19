//! Registry index

pub struct Registry;

impl Registry {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
