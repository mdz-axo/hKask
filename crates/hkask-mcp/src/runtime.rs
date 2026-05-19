//! MCP runtime

pub struct Runtime;

impl Runtime {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}
