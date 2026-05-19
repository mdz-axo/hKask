//! Agent pod lifecycle

pub struct Pod;

impl Pod {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Pod {
    fn default() -> Self {
        Self::new()
    }
}
