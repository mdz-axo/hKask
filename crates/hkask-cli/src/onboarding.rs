//! Onboarding and sign-in flow for hKask CLI
//!
//! Handles three scenarios when starting `kask chat`:
//! 1. No replicants registered — walk through creating first replicant
//! 2. One replicant — assume signing into that one, ask for passphrase
//! 3. Multiple replicants — ask which one to sign into, then passphrase
//!
//! After successful sign-in, the derived ACP secret and DB passphrase are
//! stored in the OS keychain and set as environment variables for the session,
//! so subsequent `init_registry()` calls work transparently.

use hkask_agents::AcpRuntime;
use hkask_keystore::Keychain;
use hkask_keystore::master_key::derive_all_internal_secrets;
use hkask_storage::{AgentRegistryStore, Database};
use hkask_types::{AgentDefinition, AgentKind, RegisteredAgent, WebID};
use std::sync::Arc;
use thiserror::Error;

use crate::commands::config::{self, init_registry, registry_db_path};
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

/// Outcome of the onboarding flow
pub struct OnboardingOutcome {
    /// The replicant name the user signed in as
    pub signed_in_agent: String,
}

/// Run the onboarding/sign-in flow.
///
/// Returns the replicant name the user signed in as. If the user already has
/// keys configured (HKASK_MASTER_KEY, HKASK_ACP_SECRET, or keychain entry),
/// this transparently initializes and returns without prompting.
pub async fn run_onboarding() -> Result<OnboardingOutcome, OnboardingError> {
    // First, try the fast path: if keys are already configured, just init
    match init_registry().await {
        Ok((_acp, store)) => {
            // Keys work — check if there's a replicant to sign into
            let replicants = list_replicants(&store)?;
            let agent_name = pick_or_default_replicant(&replicants)?;
            return Ok(OnboardingOutcome {
                signed_in_agent: agent_name,
            });
        }
        Err(_) => {
            // Keys not available — run interactive onboarding
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

    // Derive secrets from passphrase
    let secrets = derive_all_internal_secrets(&passphrase);

    // Store secrets in keychain and set env vars
    store_secrets(&secrets)?;

    // Now initialize the registry with the new secrets
    let (acp, store) = init_registry().await.map_err(OnboardingError::Registry)?;

    // Register the new replicant
    register_replicant(&acp, &store, &name, &description).await?;

    println!();
    println!(
        "  \x1b[32m✓\x1b[0m Replicant \x1b[1;36m{}\x1b[0m created successfully!",
        name
    );
    println!("  \x1b[32m✓\x1b[0m Keys stored in OS keychain (service: hkask)");
    println!();

    Ok(OnboardingOutcome {
        signed_in_agent: name,
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

        let secrets = derive_all_internal_secrets(&passphrase);

        // Set env vars for this attempt
        unsafe {
            std::env::set_var("HKASK_ACP_SECRET", &secrets.acp_secret);
            std::env::set_var("HKASK_DB_PASSPHRASE", &secrets.capability_key);
        }

        match init_registry().await {
            Ok((_acp, store)) => {
                // Verify the replicant exists
                match store.get(replicant_name) {
                    Ok(_) => {
                        // Success! Store in keychain for future sessions
                        store_secrets(&secrets)?;
                        println!(
                            "\n  \x1b[32m✓\x1b[0m Signed in as \x1b[1;36m{}\x1b[0m",
                            replicant_name
                        );
                        println!();
                        return Ok(OnboardingOutcome {
                            signed_in_agent: replicant_name.to_string(),
                        });
                    }
                    Err(_) => {
                        unsafe {
                            std::env::remove_var("HKASK_ACP_SECRET");
                            std::env::remove_var("HKASK_DB_PASSPHRASE");
                        }
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
                }
            }
            Err(e) => {
                unsafe {
                    std::env::remove_var("HKASK_ACP_SECRET");
                    std::env::remove_var("HKASK_DB_PASSPHRASE");
                }
                println!("  \x1b[31m✗\x1b[0m Invalid passphrase: {}", e);
                if attempt < 3 {
                    println!();
                }
            }
        }
    }

    Err(OnboardingError::InvalidPassphrase)
}

/// Register a new replicant in the agent registry
async fn register_replicant(
    acp: &Arc<AcpRuntime>,
    store: &AgentRegistryStore,
    name: &str,
    description: &str,
) -> Result<(), OnboardingError> {
    let webid = WebID::from_persona_with_namespace(name.as_bytes(), "replicant");

    // Register with ACP
    let token = acp
        .register_agent(
            webid,
            "Replicant".to_string(),
            vec![
                "tool:inference:call".to_string(),
                "tool:mcp:invoke".to_string(),
                "memory:episodic:read".to_string(),
                "memory:episodic:write".to_string(),
            ],
        )
        .await
        .map_err(|e| {
            OnboardingError::Registry(crate::errors::RegistryError::InitFailed(e.to_string()))
        })?;

    let definition = AgentDefinition {
        name: name.to_string(),
        agent_kind: AgentKind::Replicant,
        binding_contract: false,
        editor: "onboarding".to_string(),
        charter: Some(hkask_types::Charter {
            description: description.to_string(),
            archetype: String::new(),
            visibility: String::new(),
        }),
        capabilities: vec![
            "tool:inference:call".to_string(),
            "tool:mcp:invoke".to_string(),
        ],
        rights: vec![],
        responsibilities: vec![],
        reporting: None,
        standing_session: None,
        persona: None,
        depends_on: vec![],
        readiness_probe: None,
        process_manifest: None,
    };

    let registered = RegisteredAgent {
        definition,
        token_hash: token.signature.clone(),
        registered_at: chrono::Utc::now().to_rfc3339(),
        source_yaml: "onboarding".to_string(),
    };

    store.insert(&registered).map_err(|e| {
        OnboardingError::Registry(crate::errors::RegistryError::InitFailed(e.to_string()))
    })?;

    Ok(())
}

/// Store derived secrets in the OS keychain and set env vars
fn store_secrets(
    secrets: &hkask_keystore::master_key::InternalSecrets,
) -> Result<(), OnboardingError> {
    let keychain = Keychain::default();

    // Store ACP secret in keychain (used by resolve_acp_secret)
    keychain
        .store_by_key("acp-secret", &secrets.acp_secret)
        .map_err(OnboardingError::Keychain)?;

    // Store DB passphrase in keychain
    keychain
        .store_by_key("hkask-db-passphrase", &secrets.capability_key)
        .map_err(OnboardingError::Keychain)?;

    // Also set env vars for the current session
    unsafe {
        std::env::set_var("HKASK_ACP_SECRET", &secrets.acp_secret);
        std::env::set_var("HKASK_DB_PASSPHRASE", &secrets.capability_key);
    }

    Ok(())
}

/// Try to list replicants from the existing DB (without ACP secret)
fn try_list_existing_replicants() -> Result<Vec<RegisteredAgent>, OnboardingError> {
    // Try with insecure dev fallback for the DB passphrase
    let db_path = registry_db_path();
    let passphrase = config::resolve_db_passphrase().or_else(|_| {
        // If no passphrase set, try without encryption (plain SQLite)
        Ok::<String, crate::errors::RegistryError>(String::new())
    })?;

    let db = if db_path == ":memory:" || !std::path::Path::new(&db_path).exists() {
        // No DB file yet — fresh start
        return Ok(Vec::new());
    } else {
        Database::open(&db_path, &passphrase)
            .map_err(|e| OnboardingError::Database(e.to_string()))?
    };

    let store = AgentRegistryStore::new(db.conn_arc());
    store
        .initialize_schema()
        .map_err(|e| OnboardingError::Database(e.to_string()))?;

    let replicants = store
        .list_by_kind(AgentKind::Replicant)
        .map_err(|e| OnboardingError::Database(e.to_string()))?;

    Ok(replicants)
}

/// List replicants from a store
fn list_replicants(store: &AgentRegistryStore) -> Result<Vec<RegisteredAgent>, OnboardingError> {
    store
        .list_by_kind(AgentKind::Replicant)
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

// ── Interactive prompting helpers ──────────────────────────────────────────

/// Read a line of input from the user
fn read_line() -> Result<String, std::io::Error> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input)
}

/// Prompt the user and return their response (trims whitespace)
fn prompt_line(prompt: &str) -> Result<String, std::io::Error> {
    print!("{} ", prompt);
    use std::io::Write;
    std::io::stdout().flush()?;
    let line = read_line()?;
    Ok(line.trim().to_string())
}

/// Prompt for a passphrase (no echo)
fn prompt_passphrase(prompt: &str) -> Result<String, std::io::Error> {
    print!("{} ", prompt);
    use std::io::Write;
    std::io::stdout().flush()?;
    let pass = rpassword::read_password()?;
    Ok(pass)
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
            _ => {
                println!(
                    "  Please enter a number between {} and {}.",
                    range.start(),
                    range.end()
                );
            }
        }
    }
}
