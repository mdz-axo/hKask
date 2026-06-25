//! Docs command handlers for `kask docs`
//!
//! Implements the CLI display logic for documentation generation.

use crate::cli::{self, DocsAction};

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  action is a valid DocsAction variant (Openapi, Cli, All)
/// post: generates OpenAPI spec, CLI markdown reference, or both; writes to file or stdout
pub fn run(action: DocsAction) {
    match action {
        DocsAction::Openapi { output } => {
            #[cfg(not(feature = "api"))]
            let _ = output;
            #[cfg(feature = "api")]
            {
                let spec = hkask_api::create_openapi();
                let json = super::helpers::or_exit(
                    serde_json::to_string_pretty(&spec),
                    "Failed to serialize OpenAPI spec",
                );
                super::helpers::write_or_print(&json, output.as_deref(), "OpenAPI specification");
            }
            #[cfg(not(feature = "api"))]
            eprintln!("OpenAPI docs not built — rebuild with `cargo build --features api`");
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
            #[cfg(feature = "api")]
            {
                let spec = hkask_api::create_openapi();
                let json = super::helpers::or_exit(
                    serde_json::to_string_pretty(&spec),
                    "Failed to serialize OpenAPI spec",
                );
                let openapi_path = output.join("openapi.json");
                super::helpers::write_or_print(&json, Some(&openapi_path), "OpenAPI specification");
            }
            #[cfg(not(feature = "api"))]
            {
                eprintln!("OpenAPI docs not built — rebuild with `cargo build --features api`");
            }
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
