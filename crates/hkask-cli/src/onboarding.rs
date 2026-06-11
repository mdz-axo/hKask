//! Session resolution for hKask CLI.
//!
//! Two modes:
//! - **Operating mode**: keys configured, replicants exist — sign in, no prompts.
//! - **Setup**: no keys or no replicants — create the user's first replicant.
//!
//! After setup, derived secrets are stored in the OS keychain for future
//! sessions and passed directly to `init_registry_with_secrets()`.

use hkask_services::{OnboardingService, ResolvedSecrets, ServiceConfig};
use hkask_types::RegisteredAgent;
use thiserror::Error;

use crate::repl::display;

#[derive(Error, Debug)]
pub enum OnboardingError {
    #[error("Onboarding cancelled by user")]
    Cancelled,
    #[error("Registry error: {0}")]
    Registry(#[from] crate::errors::RegistryError),
    #[error("Keychain error: {0}")]
    Keychain(#[from] hkask_keystore::KeychainError),
    #[error("Database error: {0}")]
    Database(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<hkask_services::ServiceError> for OnboardingError {
    fn from(e: hkask_services::ServiceError) -> Self {
        match &e {
            hkask_services::ServiceError::Keystore(msg) => {
                // Try to downcast to KeychainError, fall back to string wrap
                OnboardingError::Database(msg.clone())
            }
            hkask_services::ServiceError::Storage(db_err) => {
                OnboardingError::Database(db_err.to_string())
            }
            hkask_services::ServiceError::AgentRegistryStore(reg_err) => {
                OnboardingError::Database(reg_err.to_string())
            }
            _ => OnboardingError::Database(e.to_string()),
        }
    }
}

/// Outcome of the onboarding flow
pub struct OnboardingOutcome {
    /// The replicant name the user signed in as
    pub signed_in_agent: String,
    /// Resolved secrets from onboarding, when available.
    ///
    /// Present when onboarding derived secrets from a passphrase (first-run
    /// or sign-in). Carries the secrets forward so the REPL can use them
    /// directly instead of re-resolving from the OS keychain (which may not
    /// persist across Entry instances with the mock backend).
    pub resolved_secrets: Option<ResolvedSecrets>,
}

/// Resolve the user's session.
///
/// Operating mode: keys configured, replicants exist — returns immediately.
/// Setup: no keys or no replicants — walks through first replicant creation.
pub async fn run_onboarding() -> Result<OnboardingOutcome, OnboardingError> {
    // Operating mode: keys work and at least one replicant exists.
    if let Ok(config) = ServiceConfig::from_env()
        && let Ok(handle) = OnboardingService::init_registry(&config).await
    {
        let replicants = list_replicants(&handle.store)?;
        if !replicants.is_empty() {
            let agent_name = if replicants.len() == 1 {
                replicants[0].definition.name.clone()
            } else {
                select_replicant(&replicants)?
            };
            return Ok(OnboardingOutcome {
                signed_in_agent: agent_name,
                resolved_secrets: None,
            });
        }
    }

    // Setup: create the user's first replicant.
    setup().await
}

/// Setup: create the user's first replicant.
async fn setup() -> Result<OnboardingOutcome, OnboardingError> {
    display::print_onboarding_banner();
    create_first_replicant_flow().await
}

/// Select which replicant to sign into when multiple exist.
fn select_replicant(replicants: &[RegisteredAgent]) -> Result<String, OnboardingError> {
    println!("\n  \x1b[1mRegistered replicants:\x1b[0m");
    for (i, r) in replicants.iter().enumerate() {
        let desc = r
            .definition
            .charter
            .as_ref()
            .map(|c| c.description.as_str())
            .unwrap_or("(no description)");
        println!(
            "    {}. \x1b[36m{}\x1b[0m — {}",
            i + 1,
            r.definition.name,
            desc
        );
    }

    let choice = prompt_choice(
        "\n  Which replicant would you like to sign in to?",
        1..=replicants.len(),
    )?;
    Ok(replicants[choice - 1].definition.name.clone())
}

/// Flow: Create the user's first replicant
async fn create_first_replicant_flow() -> Result<OnboardingOutcome, OnboardingError> {
    println!("\n  \x1b[1mWelcome to hKask!\x1b[0m");
    println!("  Let's set up your first replicant — your personal AI assistant.\n");

    // Q1: Replicant name
    let name = prompt_line("  What would you like to name your replicant?")?;
    let name = name.trim().to_string();
    if name.is_empty() {
        println!("  Name cannot be empty. Using 'Curator' as default.");
    }
    let name = if name.is_empty() {
        "Curator".to_string()
    } else {
        name
    };

    // Q2: Description / charter
    println!();
    let description = prompt_line(&format!(
        "  What should \x1b[36m{}\x1b[0m help you with? (e.g., 'coding assistant, research helper')",
        name
    ))?;
    let description = if description.trim().is_empty() {
        "A helpful AI assistant".to_string()
    } else {
        description.trim().to_string()
    };

    // Q3: Master passphrase (with confirmation)
    println!();
    println!("  Choose a \x1b[1mmaster passphrase\x1b[0m to encrypt your data.");
    println!("  This passphrase derives all your internal keys — don't lose it!");
    let passphrase = prompt_passphrase_with_confirm()?;

    // Remove orphaned DB from previous failed attempt.
    if let Ok(pre_config) = ServiceConfig::from_env()
        && OnboardingService::remove_orphaned_db(&pre_config)
    {
        eprintln!("  Removing orphaned database from previous failed setup...");
    }

    // Cleanup helper on failure.
    let cleanup = |config: &ServiceConfig| OnboardingService::cleanup_failed_onboarding(config);

    // Derive secrets and store in keychain
    let resolved = OnboardingService::derive_secrets(&passphrase, true).inspect_err(|_| {
        if let Ok(c) = ServiceConfig::from_env() {
            cleanup(&c);
        }
    })?;

    // Initialize registry with the derived secrets directly
    let config = ServiceConfig::from_secrets(
        resolved.acp_secret.clone(),
        resolved.db_passphrase.clone(),
        resolved.acp_secret.clone(), // MCP secret fallback to ACP
        name.clone(),
    );
    let handle = OnboardingService::init_registry(&config)
        .await
        .inspect_err(|_| cleanup(&config))?;

    // Register the new replicant
    OnboardingService::register_replicant(&handle.acp, &handle.store, &name, &description)
        .await
        .inspect_err(|_| cleanup(&config))?;

    println!();
    println!(
        "  \x1b[32m✓\x1b[0m Replicant \x1b[1;36m{}\x1b[0m created successfully!",
        name
    );
    println!("  \x1b[32m✓\x1b[0m Keys stored in OS keychain (service: hkask)");
    println!();

    Ok(OnboardingOutcome {
        signed_in_agent: name,
        resolved_secrets: Some(resolved),
    })
}

/// List replicants from a store
fn list_replicants(
    store: &hkask_storage::AgentRegistryStore,
) -> Result<Vec<RegisteredAgent>, OnboardingError> {
    store
        .list_by_kind(hkask_types::AgentKind::Replicant)
        .map_err(|e| OnboardingError::Database(e.to_string()))
}

/// Read a line of input from the user
fn read_line() -> Result<String, std::io::Error> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input)
}

/// Prompt the user and return their response (trims whitespace)
fn prompt_line(prompt: &str) -> Result<String, std::io::Error> {
    use std::io::Write;
    print!("{prompt} ");
    std::io::stdout().flush()?;
    read_line().map(|l| l.trim().to_string())
}

/// Prompt for a passphrase (no echo)
fn prompt_passphrase(prompt: &str) -> Result<String, std::io::Error> {
    use std::io::Write;
    print!("{prompt} ");
    std::io::stdout().flush()?;
    rpassword::read_password()
}

/// Prompt for passphrase with confirmation
fn prompt_passphrase_with_confirm() -> Result<String, std::io::Error> {
    loop {
        let pass = prompt_passphrase("  Master passphrase:")?;
        if pass.is_empty() {
            println!("  Passphrase cannot be empty. Please try again.\n");
            continue;
        }
        if pass.len() < 8 {
            println!("  Passphrase must be at least 8 characters. Please try again.\n");
            continue;
        }
        let confirm = prompt_passphrase("  Confirm passphrase:")?;
        if pass == confirm {
            return Ok(pass);
        }
        println!("  Passphrases don't match. Please try again.\n");
    }
}

/// Prompt for a numeric choice within a range
fn prompt_choice(
    prompt: &str,
    range: std::ops::RangeInclusive<usize>,
) -> Result<usize, std::io::Error> {
    loop {
        let input = prompt_line(prompt)?;
        match input.parse::<usize>() {
            Ok(n) if range.contains(&n) => return Ok(n),
            _ => println!(
                "  Please enter a number between {} and {}.",
                range.start(),
                range.end()
            ),
        }
    }
}
