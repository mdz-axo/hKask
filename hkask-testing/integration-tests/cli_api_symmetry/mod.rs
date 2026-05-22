//! CLI/API Symmetry Integration Tests
//!
//! These tests verify that CLI commands and API endpoints produce identical results
//! for equivalent operations, ensuring hexagonal port symmetry.

#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use hkask_templates::{RegistryIndex, SqliteRegistry};
    use hkask_types::TemplateType;

    /// Test template list symmetry between CLI and direct registry access
    #[test]
    fn test_template_list_cli_registry_symmetry() {
        // Create in-memory registry
        let mut registry = SqliteRegistry::new(None).expect("Failed to create registry");

        // Register a test template
        let entry = hkask_templates::RegistryEntry {
            id: "test/template".to_string(),
            template_type: TemplateType::Prompt,
            lexicon_terms: vec!["test".to_string()],
            description: "Test template".to_string(),
            source_path: "/tmp/test.j2".to_string(),
        };

        registry
            .register(entry, None)
            .expect("Failed to register template");

        // CLI output
        let cli_output = Command::cargo_bin("kask")
            .unwrap()
            .arg("template")
            .arg("list")
            .output()
            .expect("Failed to execute CLI command");

        assert!(cli_output.status.success(), "CLI command failed");

        // Verify CLI output contains the registered template
        let cli_stdout = String::from_utf8_lossy(&cli_output.stdout);
        assert!(
            cli_stdout.contains("test/template"),
            "CLI output should contain registered template"
        );

        // Direct registry access
        let entries = registry.list(None);
        assert_eq!(entries.len(), 1, "Registry should have one template");
        assert_eq!(entries[0].id, "test/template");
    }

    /// Test template search symmetry between CLI and direct registry access
    #[test]
    fn test_template_search_cli_registry_symmetry() {
        // Create in-memory registry
        let mut registry = SqliteRegistry::new(None).expect("Failed to create registry");

        // Register test templates
        let entry1 = hkask_templates::RegistryEntry {
            id: "test/selector".to_string(),
            template_type: TemplateType::Prompt,
            lexicon_terms: vec!["selector".to_string(), "routing".to_string()],
            description: "Test selector template".to_string(),
            source_path: "/tmp/selector.j2".to_string(),
        };

        let entry2 = hkask_templates::RegistryEntry {
            id: "test/action".to_string(),
            template_type: TemplateType::Process,
            lexicon_terms: vec!["action".to_string(), "execute".to_string()],
            description: "Test action template".to_string(),
            source_path: "/tmp/action.j2".to_string(),
        };

        registry
            .register(entry1, None)
            .expect("Failed to register template 1");
        registry
            .register(entry2, None)
            .expect("Failed to register template 2");

        // CLI search
        let cli_output = Command::cargo_bin("kask")
            .unwrap()
            .arg("template")
            .arg("search")
            .arg("selector")
            .output()
            .expect("Failed to execute CLI search");

        assert!(cli_output.status.success(), "CLI search failed");

        // Verify CLI output contains matching template
        let cli_stdout = String::from_utf8_lossy(&cli_output.stdout);
        assert!(
            cli_stdout.contains("test/selector"),
            "CLI search should return matching template"
        );
        assert!(
            !cli_stdout.contains("test/action"),
            "CLI search should not return non-matching template"
        );

        // Direct registry search
        let results = registry
            .search_by_lexicon("selector")
            .expect("Search failed");
        assert_eq!(results.len(), 1, "Search should return one result");
        assert_eq!(results[0].id, "test/selector");
    }

    /// Test CNS health CLI command
    #[test]
    fn test_cns_health_cli() {
        let cli_output = Command::cargo_bin("kask")
            .unwrap()
            .arg("cns")
            .arg("health")
            .output()
            .expect("Failed to execute CLI command");

        assert!(cli_output.status.success(), "CLI command failed");

        let cli_stdout = String::from_utf8_lossy(&cli_output.stdout);
        assert!(
            cli_stdout.contains("CNS health status"),
            "CLI output should contain health status header"
        );
        assert!(
            cli_stdout.contains("HEALTHY"),
            "CLI output should show healthy status"
        );
    }

    /// Test MCP servers list CLI command
    #[test]
    fn test_mcp_list_servers_cli() {
        let cli_output = Command::cargo_bin("kask")
            .unwrap()
            .arg("mcp")
            .arg("list-servers")
            .output()
            .expect("Failed to execute CLI command");

        assert!(cli_output.status.success(), "CLI command failed");

        let cli_stdout = String::from_utf8_lossy(&cli_output.stdout);
        assert!(
            cli_stdout.contains("MCP servers"),
            "CLI output should contain servers header"
        );
    }

    /// Test MCP tools list CLI command
    #[test]
    fn test_mcp_list_tools_cli() {
        let cli_output = Command::cargo_bin("kask")
            .unwrap()
            .arg("mcp")
            .arg("list-tools")
            .output()
            .expect("Failed to execute CLI command");

        assert!(cli_output.status.success(), "CLI command failed");

        let cli_stdout = String::from_utf8_lossy(&cli_output.stdout);
        assert!(
            cli_stdout.contains("Available tools"),
            "CLI output should contain tools header"
        );
    }

    /// Test pod status CLI command (placeholder)
    #[test]
    fn test_pod_status_cli() {
        let cli_output = Command::cargo_bin("kask")
            .unwrap()
            .arg("pod")
            .arg("status")
            .arg("test-pod")
            .output()
            .expect("Failed to execute CLI command");

        assert!(cli_output.status.success(), "CLI command failed");

        let cli_stdout = String::from_utf8_lossy(&cli_output.stdout);
        assert!(
            cli_stdout.contains("Agent pod status"),
            "CLI output should contain status header"
        );
        assert!(
            cli_stdout.contains("placeholder"),
            "CLI output should indicate placeholder implementation"
        );
    }

    /// Test bot list CLI command (placeholder)
    #[test]
    fn test_bot_list_cli() {
        let cli_output = Command::cargo_bin("kask")
            .unwrap()
            .arg("bot")
            .arg("list")
            .output()
            .expect("Failed to execute CLI command");

        assert!(cli_output.status.success(), "CLI command failed");

        let cli_stdout = String::from_utf8_lossy(&cli_output.stdout);
        assert!(
            cli_stdout.contains("Bot capabilities"),
            "CLI output should contain capabilities header"
        );
        assert!(
            cli_stdout.contains("ACP runtime integration"),
            "CLI output should indicate ACP integration required"
        );
    }
}
