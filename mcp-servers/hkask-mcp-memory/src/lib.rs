//! hkask-mcp-memory

pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn test_server_version() {
        assert!(!super::SERVER_VERSION.is_empty());
    }
}
