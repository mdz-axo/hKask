//! Onboarding and sign-in flow for hKask CLI
//!
//! Handles three scenarios when starting `kask chat`:
//! 1. No replicants registered — walk through creating first replicant
//! 2. One replicant — assume signing into that one, ask for passphrase
//! 3. Multiple replicants — ask which one to sign into, then passphrase
//!
//! After successful sign-in, the derived ACP secret and DB passphrase are
//! stored in the OS keychain for future sessions, and passed directly to
//! `init_registry_with_secrets()` so no runtime env mutation is needed.

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
    #[error("Invalid passphrase")]
    InvalidPassphrase,
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

/// Run the onboarding flow.
///
/// Returns the replicant name the user signed in as. If the user already has
/// keys configured (HKASK_MASTER_KEY, keychain entry, or HKASK_ACP_SECRET),
/// this transparently initializes and returns without prompting.
pub async fn run_onboarding() -> Result<OnboardingOutcome, OnboardingError> {
    // First, try the fast path: if keys are already configured, just init
    match ServiceConfig::from_env() {
        Ok(config) => match OnboardingService::init_registry(&config).await {
            Ok(handle) => {
                // Keys work — check if there's a replicant to sign into
                let replicants = list_replicants(&handle.store)?;
                let agent_name = pick_or_default_replicant(&replicants)?;
                return Ok(OnboardingOutcome {
                    signed_in_agent: agent_name,
                    resolved_secrets: None,
                });
            }
            Err(_) => {
                // Keys not available — run interactive onboarding
            }
        },
        Err(_) => {
            // Config resolution failed — run interactive onboarding
        }
    }

    // Interactive onboarding path
    interactive_onboarding().await
}

/// Interactive onboarding: collect passphrase, set up keys, handle replicants
async fn interactive_onboarding() -> Result<OnboardingOutcome, OnboardingError> {
    display::print_onboarding_banner();

    // Step 1: Try to open the DB to see if replicants already exist
    let existing_replicants = try_list_existing_replicants().unwrap_or_default();

    if existing_replicants.is_empty() {
        // Scenario 1: No replicants — create first one
        create_first_replicant_flow().await
    } else if existing_replicants.len() == 1 {
        // Scenario 2: One replicant — sign into it
        let replicant = &existing_replicants[0];
        let name = replicant.definition.name.clone();
        println!("\n  Found replicant: \x1b[1;36m{}\x1b[0m", name);
        sign_in_flow(&name).await
    } else {
        // Scenario 3: Multiple replicants — pick one
        println!("\n  \x1b[1mRegistered replicants:\x1b[0m");
        for (i, r) in existing_replicants.iter().enumerate() {
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
            1..=existing_replicants.len(),
        )?;
        let name = existing_replicants[choice - 1].definition.name.clone();
        sign_in_flow(&name).await
    }
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
    let resolved = OnboardingService::derive_and_store_secrets(&passphrase).inspect_err(|_| {
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

/// Flow: Sign into an existing replicant with a passphrase
async fn sign_in_flow(replicant_name: &str) -> Result<OnboardingOutcome, OnboardingError> {
    println!();

    // Try up to 3 times
    for attempt in 1..=3 {
        let passphrase = prompt_passphrase(&format!(
            "  Enter master passphrase for \x1b[36m{}\x1b[0m (attempt {}/3):",
            replicant_name, attempt
        ))?;

        let resolved = OnboardingService::derive_secrets(&passphrase);

        let config = ServiceConfig::from_secrets(
            resolved.acp_secret.clone(),
            resolved.db_passphrase.clone(),
            resolved.acp_secret.clone(),
            replicant_name.to_string(),
        );

        match OnboardingService::try_sign_in(&config, replicant_name, &resolved).await {
            Ok(outcome) => {
                println!(
                    "\n  \x1b[32m✓\x1b[0m Signed in as \x1b[1;36m{}\x1b[0m",
                    replicant_name
                );
                println!();
                return Ok(OnboardingOutcome {
                    signed_in_agent: outcome.agent_name,
                    resolved_secrets: Some(outcome.resolved_secrets),
                });
            }
            Err(hkask_services::ServiceError::AgentNotFound(_)) => {
                println!(
                    "  \x1b[31m✗\x1b[0m Replicant '{}' not found with this passphrase.",
                    replicant_name
                );
                if attempt < 3 {
                    println!(
                        "  (The passphrase may belong to a different replicant's database.)\n"
                    );
                }
            }
            Err(e) => {
                println!("  \x1b[31m✗\x1b[0m Invalid passphrase: {}", e);
                if attempt < 3 {
                    println!();
                }
            }
        }
    }

    Err(OnboardingError::InvalidPassphrase)
}

/// Try to list replicants from the existing DB (without ACP secret)
fn try_list_existing_replicants() -> Result<Vec<RegisteredAgent>, OnboardingError> {
    let config = match ServiceConfig::from_env() {
        Ok(c) => c,
        Err(_) => return Ok(Vec::new()), // No config — fresh start
    };

    Ok(OnboardingService::try_list_existing_replicants(&config))
}

/// List replicants from a store
fn list_replicants(
    store: &hkask_storage::AgentRegistryStore,
) -> Result<Vec<RegisteredAgent>, OnboardingError> {
    store
        .list_by_kind(hkask_types::AgentKind::Replicant)
        .map_err(|e| OnboardingError::Database(e.to_string()))
}

/// Pick a replicant from the list (auto-select if only one, ask if multiple)
fn pick_or_default_replicant(replicants: &[RegisteredAgent]) -> Result<String, OnboardingError> {
    if replicants.is_empty() {
        // No replicants but keys work — default to "Curator" (will be created on first chat)
        Ok("Curator".to_string())
    } else if replicants.len() == 1 {
        Ok(replicants[0].definition.name.clone())
    } else {
        // Multiple — but we're on the fast path (keys pre-configured), just pick first
        Ok(replicants[0].definition.name.clone())
    }
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
