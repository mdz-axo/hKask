//! Docs command handlers for `kask docs`
//!
//! Implements the CLI display logic for documentation generation.

use crate::cli::{self, DocsAction};

/// REQ: CLI-078
/// pre:  action is a valid DocsAction variant (Openapi, Cli, All)
/// post: generates OpenAPI spec, CLI markdown reference, or both; writes to file or stdout
pub fn run(action: DocsAction) {
    match action {
        DocsAction::Openapi { output } => {
            let spec = hkask_api::create_openapi();
            let json = super::helpers::or_exit(
                serde_json::to_string_pretty(&spec),
                "Failed to serialize OpenAPI spec",
            );
            super::helpers::write_or_print(&json, output.as_deref(), "OpenAPI specification");
        }
        DocsAction::Cli { output } => {
            let help = cli::generate_cli_markdown();
            super::helpers::write_or_print(&help, output.as_deref(), "CLI documentation");
        }
        DocsAction::All { output } => {
            super::helpers::or_exit(
                std::fs::create_dir_all(&output),
                "Failed to create output directory",
            );
            let spec = hkask_api::create_openapi();
            let json = super::helpers::or_exit(
                serde_json::to_string_pretty(&spec),
                "Failed to serialize OpenAPI spec",
            );
            let openapi_path = output.join("openapi.json");
            super::helpers::write_or_print(&json, Some(&openapi_path), "OpenAPI specification");
            let help = cli::generate_cli_markdown();
            let cli_path = output.join("cli-reference.md");
            super::helpers::write_or_print(&help, Some(&cli_path), "CLI documentation");
            println!(
                "\nDocumentation generated successfully in: {}",
                output.display()
            );
        }
    }
}
