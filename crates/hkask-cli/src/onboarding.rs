//! Session resolution for hKask CLI.
//!
//! Two modes:
//! - **Operating mode**: keys configured, replicants exist — sign in, no prompts.
//! - **Setup**: no keys or no replicants — create the user's first replicant.
//!
//! Also exposes `run_add_replicant()` for the `kask onboard` subcommand, which
//! creates additional replicants in an existing installation.
//!
//! After setup, derived secrets are stored in the OS keychain for future
//! sessions and passed directly to `init_registry_with_secrets()`.

use hkask_services::{OnboardingService, ResolvedSecrets, ServiceConfig};
use hkask_types::{RegisteredAgent, UserProfile};
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
            hkask_services::ServiceError::Keystore(msg) => OnboardingError::Database(msg.clone()),
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
    /// The model selected during onboarding (first-run only).
    /// When present, the REPL uses this instead of the hardcoded default.
    pub selected_model: Option<String>,
    /// Whether this is a first-run (true) or a returning session (false).
    /// Used by the REPL to decide whether to show the First Steps guide.
    pub is_first_run: bool,
}

// ── Public entry points ────────────────────────────────────────────────────

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
                selected_model: None,
                is_first_run: false,
            });
        }
    }

    // Setup: create the user's first replicant.
    setup().await
}

/// Add a new replicant to an existing hKask installation.
///
/// Used by `kask onboard`. When secrets are already in the keychain the user
/// only provides name + description (no passphrase re-entry needed). When
/// secrets are absent the full passphrase flow runs, matching first-run.
pub async fn run_add_replicant() -> Result<(), OnboardingError> {
    display::print_onboarding_banner();
    println!("\n  \x1b[1mAdd a new replicant\x1b[0m");
    println!("  Each replicant is a distinct AI identity with its own memory and charter.\n");

    // Require existing secrets from the keychain — `kask onboard` adds to an existing
    // installation, it does not bootstrap one.
    let config = ServiceConfig::from_env().map_err(|_| {
        eprintln!("  \x1b[31m✗\x1b[0m No hKask installation found in OS keychain.");
        eprintln!("  Run \x1b[36mkask chat\x1b[0m first to complete initial setup, then use");
        eprintln!("  \x1b[36mkask onboard\x1b[0m to add additional replicants.");
        OnboardingError::Database("No keychain secrets — run `kask chat` first".to_string())
    })?;

    // Open the existing registry.
    let handle = OnboardingService::init_registry(&config)
        .await
        .map_err(|e| {
            eprintln!("  \x1b[31m✗\x1b[0m Cannot open registry: {}", e);
            eprintln!("  Make sure you have completed first-run setup (`kask chat`).");
            e
        })?;

    // Load the user profile for naming protocol
    let user_profile = OnboardingService::get_user_profile(&handle.store).map_err(|e| {
        eprintln!("  \x1b[31m✗\x1b[0m Cannot read user profile: {}", e);
        e
    })?;

    // Q1: Replicant first name
    if let Some(ref profile) = user_profile {
        println!(
            "  Your replicant's full name will be \x1b[36m[chosen] r{}\x1b[0m.",
            profile.last_name
        );
    }
    let name = prompt_line("  Replicant first name:")?;
    let name = if name.trim().is_empty() {
        println!("  Name cannot be empty. Using 'Assistant' as default.");
        "Assistant".to_string()
    } else {
        name.trim().to_string()
    };
    let display_name = if let Some(ref profile) = user_profile {
        profile.replicant_display_name(&name)
    } else {
        name.clone()
    };

    // Q2: Tag line
    println!();
    let description = prompt_line(&format!(
        "  Tag line for \x1b[36m{}\x1b[0m: (e.g., 'research assistant'):",
        display_name
    ))?;
    let description = if description.trim().is_empty() {
        "A helpful AI assistant".to_string()
    } else {
        description.trim().to_string()
    };

    // Q3: Model selection
    println!();
    println!("  \x1b[1mChoose a model\x1b[0m for this replicant.");
    let selected_model = select_model().await;

    OnboardingService::register_replicant(
        &handle.acp,
        &handle.store,
        &name,
        &description,
        user_profile.as_ref(),
        None,
        None,
    )
    .await
    .map_err(|e| {
        eprintln!("  \x1b[31m✗\x1b[0m Failed to register replicant: {}", e);
        e
    })?;

    // Summary
    println!();
    println!("  \x1b[1;32m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
    println!("  \x1b[1;32m  ✓  Replicant added!\x1b[0m");
    println!("  \x1b[1;32m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
    println!();
    println!(
        "  \x1b[1mReplicant:\x1b[0m  \x1b[36m{}\x1b[0m",
        display_name
    );
    println!("  \x1b[1mTag line:\x1b[0m  {}", description);
    println!(
        "  \x1b[1mModel:\x1b[0m     \x1b[36m{}\x1b[0m",
        selected_model
    );
    println!();
    println!(
        "  Start a session: \x1b[36mkask chat {}\x1b[0m",
        display_name
    );
    println!();

    Ok(())
}

// ── Private helpers ────────────────────────────────────────────────────────

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
    println!("  Let's set up your profile and your first replicant.\n");

    // ── Human identity (collected first — replicant naming depends on it) ──

    // Q1: Human first name
    let human_first = prompt_line("  What is your first name?")?;
    let human_first = human_first.trim().to_string();
    if human_first.is_empty() {
        println!("  Name cannot be empty. Please enter your first name.");
        return Err(OnboardingError::Cancelled);
    }

    // Q2: Human last name
    println!();
    let human_last = prompt_line("  What is your last name?")?;
    let human_last = human_last.trim().to_string();
    if human_last.is_empty() {
        println!("  Name cannot be empty. Please enter your last name.");
        return Err(OnboardingError::Cancelled);
    }

    // Q3: Human email
    println!();
    let human_email = prompt_line("  Your email address:")?;
    let human_email = human_email.trim().to_string();

    let user_profile = UserProfile {
        first_name: human_first.clone(),
        last_name: human_last.clone(),
        email: human_email,
    };

    // ── Replicant creation ──

    // Q5: Replicant first name (naming protocol explained)
    println!();
    println!("  \x1b[1mNow, name your replicant.\x1b[0m");
    println!(
        "  Your replicant's full name will be \x1b[36m[chosen] r{}\x1b[0m —",
        human_last
    );
    println!("  so you always know it's your assistant.");
    let name = prompt_line("  What first name should your replicant have?")?;
    let name = name.trim().to_string();
    if name.is_empty() {
        println!("  Name cannot be empty. Using 'Curator' as default.");
    }
    let name = if name.is_empty() {
        "Curator".to_string()
    } else {
        name
    };
    let display_name = user_profile.replicant_display_name(&name);

    // Q6: Tag line
    println!();
    let description = prompt_line(&format!(
        "  Tag line for \x1b[36m{}\x1b[0m: (e.g., 'finance assistant, research helper')",
        display_name
    ))?;
    let description = if description.trim().is_empty() {
        "A helpful AI assistant".to_string()
    } else {
        description.trim().to_string()
    };

    // ── Provider setup (before model selection — models need a provider) ──
    println!();
    setup_provider().await?;

    // Q7: Model selection
    println!();
    println!("  \x1b[1mChoose a model\x1b[0m for your replicant to use.");
    println!("  Models determine how your replicant thinks and responds.");
    let selected_model = select_model().await;

    // Q8: Master passphrase (with confirmation)
    println!();
    println!("  Choose a \x1b[1mmaster passphrase\x1b[0m to encrypt your data.");
    println!("  This passphrase derives all your internal security keys.");
    println!("  \x1b[2mStore it in a password manager — it cannot be recovered if lost.\x1b[0m");
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
    let resolved = OnboardingService::derive_secrets(&passphrase, true).inspect_err(|e| {
        eprintln!("  \x1b[31m✗\x1b[0m Failed to derive security keys: {}", e);
        eprintln!(
            "  This may indicate a keychain access issue. Try running with appropriate permissions."
        );
        if let Ok(c) = ServiceConfig::from_env() {
            cleanup(&c);
        }
    })?;

    // Initialize registry with the derived secrets directly
    let config = ServiceConfig::from_secrets(
        resolved.acp_secret.clone(),
        resolved.db_passphrase.clone(),
        resolved.acp_secret.clone(), // MCP secret fallback to ACP
        display_name.clone(),
    );
    let handle = OnboardingService::init_registry(&config)
        .await
        .inspect_err(|e| {
            eprintln!("  \x1b[31m✗\x1b[0m Failed to initialize database: {}", e);
            eprintln!("  Check disk space and permissions, then run `kask chat` to retry.");
            cleanup(&config);
        })?;

    // Store the user profile
    OnboardingService::store_user_profile(&handle.store, &user_profile).inspect_err(|e| {
        eprintln!("  \x1b[31m✗\x1b[0m Failed to store user profile: {}", e);
        cleanup(&config);
    })?;

    // Register the new replicant with naming protocol applied
    OnboardingService::register_replicant(
        &handle.acp,
        &handle.store,
        &name,
        &description,
        Some(&user_profile),
        None,
        None,
    )
    .await
    .inspect_err(|e| {
        eprintln!("  \x1b[31m✗\x1b[0m Failed to register replicant: {}", e);
        eprintln!("  Run `kask chat` to retry onboarding.");
        cleanup(&config);
    })?;

    // Post-creation summary
    print_creation_summary(&display_name, &description, &selected_model);

    Ok(OnboardingOutcome {
        signed_in_agent: display_name,
        resolved_secrets: Some(resolved),
        selected_model: Some(selected_model),
        is_first_run: true,
    })
}

/// Curated list of cloud near-frontier models for replicant cognition.
/// All available via DeepInfra by default.
const ONBOARDING_MODELS: &[&str] = &[
    "deepseek-v4-pro",
    "GLM5.1",
    "Qwen3.5 397B",
    "Kimi2.6",
    "minimax-m3",
    "nemotron-3-super",
    "gemma4",
];

/// Let the user select a model from the curated cloud frontier list.
async fn select_model() -> String {
    let default_model = hkask_inference::InferenceConfig::from_env().default_model;

    println!("  \x1b[1mAvailable models\x1b[0m (via DeepInfra):");
    for (i, name) in ONBOARDING_MODELS.iter().enumerate() {
        let marker = if *name == default_model {
            " (default)"
        } else {
            ""
        };
        println!("    {}. \x1b[36m{}\x1b[0m{}", i + 1, name, marker);
    }
    println!(
        "    {}. Enter a model name manually",
        ONBOARDING_MODELS.len() + 1
    );

    let choice = prompt_choice(
        &format!(
            "  Select a model (1-{}, default: \x1b[36m{}\x1b[0m):",
            ONBOARDING_MODELS.len() + 1,
            default_model
        ),
        1..=(ONBOARDING_MODELS.len() + 1),
    );

    match choice {
        Ok(n) if n <= ONBOARDING_MODELS.len() => ONBOARDING_MODELS[n - 1].to_string(),
        Ok(_) => {
            let input = prompt_line("  Model name:");
            match input {
                Ok(s) if s.trim().is_empty() => default_model,
                Ok(s) => s.trim().to_string(),
                Err(_) => default_model,
            }
        }
        Err(_) => default_model,
    }
}

/// Interactive provider setup during first-run onboarding.
///
/// Checks if a provider API key is already configured (env var or keychain).
/// If not, prompts the user to load from a providers.env file, enter a key
/// directly, or skip (Ollama-only, with a warning).
async fn setup_provider() -> Result<(), OnboardingError> {
    let config = hkask_inference::InferenceConfig::from_env();

    // Check if any cloud provider is already configured
    let has_deepinfra = !config.deepinfra_api_key.is_empty();
    let has_fireworks = !config.fireworks_api_key.is_empty();
    let has_fal = !config.fal_api_key.is_empty();

    if has_deepinfra || has_fireworks || has_fal {
        let provider_name = if has_deepinfra {
            "DeepInfra"
        } else if has_fireworks {
            "Fireworks"
        } else {
            "fal.ai"
        };
        println!(
            "  \x1b[32m✓\x1b[0m {} API key found — using {} as default provider.",
            provider_name, provider_name
        );
        return Ok(());
    }

    // No cloud provider configured — prompt the user
    println!("  \x1b[1mInference provider\x1b[0m");
    println!();
    println!("  hKask needs a cloud provider to think. Without one,");
    println!("  your replicant cannot respond to you.");
    println!();
    println!("  You can set this up now, or later with:");
    println!();
    println!("    \x1b[36mkask keystore load --path providers.env --shred\x1b[0m");
    println!();
    println!("  How would you like to configure?");
    println!("    1. Load from providers.env file");
    println!("    2. Enter API key directly");
    println!("    3. Skip for now (requires Ollama running locally)");
    println!();

    let choice = prompt_choice("  Choice (1-3):", 1..=3)?;

    match choice {
        1 => {
            // Load from file
            let path_str = prompt_line("  Path to providers.env:")?;
            let path_str = path_str.trim();
            if path_str.is_empty() {
                println!("  No path entered — skipping provider setup.");
                println!("  Run `kask keystore load --path providers.env --shred` later.");
                return Ok(());
            }
            let path = std::path::PathBuf::from(path_str);
            if !path.exists() {
                println!("  \x1b[31m✗\x1b[0m File not found: {}", path.display());
                println!("  Create it from the template: cp providers.env.example providers.env");
                println!("  Then fill in your API keys and run `kask chat` again.");
                return Err(OnboardingError::Cancelled);
            }

            // Parse the file to check it has keys
            let content = std::fs::read_to_string(&path).map_err(|e| {
                eprintln!("  \x1b[31m✗\x1b[0m Cannot read {}: {}", path.display(), e);
                OnboardingError::Io(e)
            })?;

            let mut found_keys: Vec<&str> = Vec::new();
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    if !value.trim().is_empty()
                        && (key.trim() == "DI_API_KEY"
                            || key.trim() == "FW_API_KEY"
                            || key.trim() == "FA_API_KEY")
                    {
                        found_keys.push(key.trim());
                    }
                }
            }

            if found_keys.is_empty() {
                println!(
                    "  \x1b[31m✗\x1b[0m No API keys found in {}.",
                    path.display()
                );
                println!("  Fill in at least one of DI_API_KEY, FW_API_KEY, or FA_API_KEY.");
                return Err(OnboardingError::Cancelled);
            }

            println!(
                "  Found {} provider key(s): {}",
                found_keys.len(),
                found_keys.join(", ")
            );

            // Store in keychain
            let keychain = hkask_keystore::Keychain::default();
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    if value.is_empty() {
                        continue;
                    }
                    // Store all non-empty keys (not just API keys — also HKASK_DEFAULT_PROVIDER, etc.)
                    if let Err(e) = keychain.store_by_key(key, value) {
                        eprintln!("  \x1b[31m✗\x1b[0m Failed to store {}: {}", key, e);
                    }
                }
            }
            println!("  \x1b[32m✓\x1b[0m Keys stored in OS keychain.");

            // Shred with consent
            println!();
            println!("  ═══════════════════════════════════════════════════════════");
            println!(
                "  ⚠️  The file {} will now be permanently deleted.",
                path.display()
            );
            println!("  Make sure you have a backup of your API keys.");
            println!("  ═══════════════════════════════════════════════════════════");
            println!();

            let shred_choice = prompt_choice(
                &format!(
                    "  Delete {}? [1=yes, I have a backup / 2=no, keep the file]:",
                    path.display()
                ),
                1..=2,
            )?;

            if shred_choice == 1 {
                // Simple overwrite + delete (same logic as keystore command)
                if let Ok(metadata) = std::fs::metadata(&path) {
                    let len = metadata.len().min(65536) as usize;
                    let mut random_bytes = vec![0u8; len];
                    rand::rng().fill_bytes(&mut random_bytes);
                    let _ = std::fs::write(&path, &random_bytes);
                }
                let _ = std::fs::remove_file(&path);
                println!("  \x1b[32m✓\x1b[0m File shredded.");
            } else {
                println!("  File kept on disk — delete it yourself when ready.");
            }
        }
        2 => {
            // Enter API key directly
            println!();
            println!("  Supported providers:");
            println!("    DI — DeepInfra (recommended, wide model catalog)");
            println!("    FW — Fireworks.ai (fast serverless inference)");
            println!("    FA — fal.ai (specialized vision/OCR models)");
            println!();

            let provider_str = prompt_line("  Provider code (DI/FW/FA):")?;
            let provider_str = provider_str.trim().to_uppercase();

            let key_name = match provider_str.as_str() {
                "DI" => "DI_API_KEY",
                "FW" => "FW_API_KEY",
                "FA" => "FA_API_KEY",
                _ => {
                    println!(
                        "  \x1b[31m✗\x1b[0m Unknown provider '{}'. Use DI, FW, or FA.",
                        provider_str
                    );
                    return Err(OnboardingError::Cancelled);
                }
            };

            let api_key = prompt_line(&format!("  {} API key:", key_name))?;
            let api_key = api_key.trim();
            if api_key.is_empty() {
                println!("  No key entered — skipping provider setup.");
                return Ok(());
            }

            let keychain = hkask_keystore::Keychain::default();
            keychain.store_by_key(key_name, api_key).map_err(|e| {
                eprintln!("  \x1b[31m✗\x1b[0m Failed to store key: {}", e);
                OnboardingError::Keychain(e)
            })?;

            // Also set default provider to match
            let _ = keychain.store_by_key("HKASK_DEFAULT_PROVIDER", &provider_str);

            println!("  \x1b[32m✓\x1b[0m Key stored in OS keychain.");
            println!("  Default provider set to {}.", provider_str);
        }
        3 => {
            println!();
            println!("  \x1b[33m⚠\x1b[0m  Skipping cloud provider setup.");
            println!("  hKask will use Ollama (local inference).");
            println!("  Make sure Ollama is running at http://127.0.0.1:11434");
            println!();
            println!("  To add a cloud provider later:");
            println!("    \x1b[36mkask keystore load --path providers.env --shred\x1b[0m");
        }
        _ => unreachable!(),
    }

    Ok(())
}

/// Print a summary after successful replicant creation (first-run).
fn print_creation_summary(name: &str, description: &str, model: &str) {
    println!();
    println!("  \x1b[1;32m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
    println!("  \x1b[1;32m  ✓  Replicant created successfully!\x1b[0m");
    println!("  \x1b[1;32m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
    println!();
    println!("  \x1b[1mReplicant:\x1b[0m  \x1b[36m{}\x1b[0m", name);
    println!("  \x1b[1mTag line:\x1b[0m  {}", description);
    println!("  \x1b[1mModel:\x1b[0m     \x1b[36m{}\x1b[0m", model);
    println!("  \x1b[1mSecurity:\x1b[0m  Keys stored in OS keychain (encrypted DB)");
    println!();
    println!("  \x1b[1mGetting started:\x1b[0m");
    println!("  • Just type to chat with your replicant");
    println!("  • \x1b[36m/help\x1b[0m   — see all available commands");
    println!("  • \x1b[36m/model\x1b[0m  — switch models anytime");
    println!("  • \x1b[36m/tools\x1b[0m  — discover available MCP tools");
    println!("  • \x1b[36m/start\x1b[0m  — take a guided tour of hKask");
    println!();
    println!("  \x1b[2mTry asking: \"What can you help me with?\"\x1b[0m");
    println!();
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

/// Evaluate passphrase strength and return a label + color code.
fn passphrase_strength(pass: &str) -> (&'static str, &'static str) {
    let len = pass.len();
    let has_upper = pass.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = pass.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = pass.chars().any(|c| c.is_ascii_digit());
    let has_special = pass.chars().any(|c| !c.is_alphanumeric());
    let variety = [has_upper, has_lower, has_digit, has_special]
        .iter()
        .filter(|&&x| x)
        .count();

    if len >= 16 && variety >= 3 {
        ("strong", "\x1b[32m") // green
    } else if len >= 12 && variety >= 2 {
        ("good", "\x1b[33m") // yellow
    } else if len >= 8 {
        ("fair", "\x1b[33m") // yellow
    } else {
        ("weak", "\x1b[31m") // red
    }
}

/// Prompt for passphrase with confirmation and strength feedback
fn prompt_passphrase_with_confirm() -> Result<String, std::io::Error> {
    loop {
        let pass = prompt_passphrase("  Master passphrase:")?;
        if pass.is_empty() {
            println!("  \x1b[31mPassphrase cannot be empty.\x1b[0m Please try again.\n");
            continue;
        }
        if pass.len() < 8 {
            println!(
                "  \x1b[31mPassphrase must be at least 8 characters.\x1b[0m Please try again.\n"
            );
            continue;
        }
        // Show strength feedback
        let (label, color) = passphrase_strength(&pass);
        println!("  Passphrase strength: {color}{label}\x1b[0m");

        let confirm = prompt_passphrase("  Confirm passphrase:")?;
        if pass == confirm {
            return Ok(pass);
        }
        println!("  \x1b[31mPassphrases don't match.\x1b[0m Please try again.\n");
    }
}

/// Prompt for a numeric choice within a range
fn prompt_choice(
    prompt: &str,
    range: std::ops::RangeInclusive<usize>,
) -> Result<usize, std::io::Error> {
    loop {
        let input = prompt_line(prompt)?;
        if input.trim().is_empty() {
            // Default to first option on empty input
            return Ok(*range.start());
        }
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

#[cfg(test)]
mod tests {
    use super::passphrase_strength;

    // REQ: Passphrases shorter than 8 characters are classified "weak"
    // regardless of character variety.
    #[test]
    fn passphrase_strength_weak_below_8() {
        assert_eq!(passphrase_strength("Ab1!").0, "weak");
        assert_eq!(passphrase_strength("abcdefg").0, "weak"); // exactly 7
        assert_eq!(passphrase_strength("").0, "weak");
    }

    // REQ: An 8-character passphrase with only one character class (lowercase
    // letters) is classified "fair" — meets the minimum length but lacks variety.
    #[test]
    fn passphrase_strength_fair_at_8_single_variety() {
        // 8 chars, lowercase only → variety = 1 → fair
        assert_eq!(passphrase_strength("abcdefgh").0, "fair");
        // 11 chars, still only one class → still fair (not enough variety for "good")
        assert_eq!(passphrase_strength("abcdefghijk").0, "fair");
    }

    // REQ: A 16-character passphrase with at least 3 character classes is
    // classified "strong".
    #[test]
    fn passphrase_strength_strong_at_16_high_variety() {
        // 16 chars: upper + lower + digit + special → variety = 4 → strong
        assert_eq!(passphrase_strength("Abcdefgh1!xyz123").0, "strong");
        // 16 chars: upper + lower + digit (3 classes) → also strong
        assert_eq!(passphrase_strength("Abcdefgh1zzz1234").0, "strong");
    }
}
