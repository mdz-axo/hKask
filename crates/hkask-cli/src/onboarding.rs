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

use hkask_services::{
    InferenceContext, InferenceService, ModelInfo, OnboardingService, ResolvedSecrets,
    ServiceConfig, get_messaging_profile, order_number, search_available_numbers, send_welcome_sms,
    verify_api_key,
};
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

    // Q2: Description / charter
    println!();
    let description = prompt_line(&format!(
        "  What should \x1b[36m{}\x1b[0m help you with? (e.g., 'research assistant'):",
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

    // ── Telnyx: phone + WhatsApp setup (optional) ──
    if let Some(ref profile) = user_profile {
        if !profile.phone.is_empty() {
            println!();
            println!(
                "  \x1b[1mPhone & WhatsApp\x1b[0m — {} can reach you via SMS, WhatsApp, and calls.",
                display_name
            );
            let enable =
                prompt_line("  Set this up now? (requires a funded Telnyx account) [y/N]:")?;
            if enable.trim().to_lowercase() == "y" {
                match setup_telnyx_for_replicant(&display_name, &profile.phone).await {
                    Ok((phone, whatsapp)) => {
                        let mut agent = match handle.store.get(&display_name) {
                            Ok(a) => a,
                            Err(e) => {
                                eprintln!("  \x1b[31m✗\x1b[0m Failed to read back replicant: {e}");
                                return Err(OnboardingError::Database(format!(
                                    "Failed to read back replicant: {e}"
                                )));
                            }
                        };
                        agent.definition.phone_number = Some(phone);
                        agent.definition.whatsapp_id = Some(whatsapp);
                        if let Err(e) = handle.store.insert(&agent) {
                            eprintln!("  \x1b[31m✗\x1b[0m Failed to update replicant: {e}");
                            return Err(OnboardingError::Database(format!(
                                "Failed to update replicant: {e}"
                            )));
                        }
                    }
                    Err(e) => {
                        eprintln!("  \x1b[33m⚠\x1b[0m  Telnyx setup skipped: {e}");
                        eprintln!(
                            "  You can set up phone/WhatsApp later via `kask pod assign {} telnyx`.",
                            display_name
                        );
                    }
                }
            }
        }
    }

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
    println!("  \x1b[1mPurpose:\x1b[0m   {}", description);
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

/// Fetch available models from inference providers for the onboarding model selection step.
/// Returns up to 8 models, sorted with smaller/faster models first.
async fn fetch_models_for_onboarding() -> Vec<ModelInfo> {
    let inference_config = hkask_inference::InferenceConfig::from_env();
    let ctx = InferenceContext::from_parts(
        None, // No shared port during onboarding
        "deepseek-v4-pro",
        inference_config,
    );
    match InferenceService::list_models(&ctx).await {
        Ok(models) => {
            // Sort: smaller models first (easier for new users to run)
            let mut sorted = models;
            sorted.sort_by_key(|m| m.size_bytes.unwrap_or(u64::MAX));
            sorted.truncate(8);
            sorted
        }
        Err(_) => Vec::new(),
    }
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

    // Q3: Human phone
    println!();
    println!("  Your phone number lets your replicant reach you via call, SMS, or WhatsApp.");
    let human_phone = prompt_line("  Your phone number (e.g., +15551234567):")?;
    let human_phone = human_phone.trim().to_string();
    if human_phone.is_empty() {
        println!("  Skipping phone — you can add it later via settings.");
    }
    let human_phone = if human_phone.is_empty() {
        String::new()
    } else {
        human_phone
    };

    // Q4: Human email
    println!();
    let human_email = prompt_line("  Your email address:")?;
    let human_email = human_email.trim().to_string();

    let user_profile = UserProfile {
        first_name: human_first.clone(),
        last_name: human_last.clone(),
        phone: human_phone.clone(),
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

    // Q6: Description / charter
    println!();
    let description = prompt_line(&format!(
        "  What should \x1b[36m{}\x1b[0m help you with? (e.g., 'coding assistant, research helper')",
        display_name
    ))?;
    let description = if description.trim().is_empty() {
        "A helpful AI assistant".to_string()
    } else {
        description.trim().to_string()
    };

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
        None, // phone_number — set post-creation via Telnyx setup
        None, // whatsapp_id — set post-creation via Telnyx setup
    )
    .await
    .inspect_err(|e| {
        eprintln!("  \x1b[31m✗\x1b[0m Failed to register replicant: {}", e);
        eprintln!("  Run `kask chat` to retry onboarding.");
        cleanup(&config);
    })?;

    // ── Telnyx: phone + WhatsApp setup (optional) ──

    if !user_profile.phone.is_empty() {
        println!();
        println!(
            "  \x1b[1mPhone & WhatsApp\x1b[0m — {} can reach you via SMS, WhatsApp, and calls.",
            display_name
        );
        let enable = prompt_line("  Set this up now? (requires a funded Telnyx account) [y/N]:")?;
        if enable.trim().to_lowercase() == "y" {
            match setup_telnyx_for_replicant(&display_name, &user_profile.phone).await {
                Ok((phone, whatsapp)) => {
                    // Update the replicant's definition with the new phone number
                    let mut agent = handle.store.get(&display_name).map_err(|e| {
                        OnboardingError::Database(format!("Failed to read back replicant: {e}"))
                    })?;
                    agent.definition.phone_number = Some(phone);
                    agent.definition.whatsapp_id = Some(whatsapp);
                    handle.store.insert(&agent).map_err(|e| {
                        OnboardingError::Database(format!("Failed to update replicant: {e}"))
                    })?;
                }
                Err(e) => {
                    eprintln!("  \x1b[33m⚠\x1b[0m  Telnyx setup skipped: {e}");
                    eprintln!(
                        "  You can set up phone/WhatsApp later via `kask pod assign {} telnyx`.",
                        display_name
                    );
                }
            }
        }
    }

    // Post-creation summary
    print_creation_summary(&display_name, &description, &selected_model);

    Ok(OnboardingOutcome {
        signed_in_agent: display_name,
        resolved_secrets: Some(resolved),
        selected_model: Some(selected_model),
        is_first_run: true,
    })
}

/// Provision a phone number and send a welcome SMS for a newly created replicant.
///
/// Returns (phone_number, whatsapp_id) on success. Both are the same E.164 number.
async fn setup_telnyx_for_replicant(
    replicant_name: &str,
    user_phone: &str,
) -> Result<(String, String), OnboardingError> {
    // Check for API key
    let api_key = match std::env::var("HKASK_TELNYX_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            return Err(OnboardingError::Database(
                "HKASK_TELNYX_API_KEY not set. Add it to your environment (see env.example) and try again.".to_string(),
            ));
        }
    };

    // Verify the key works
    println!("  Verifying Telnyx API key...");
    if !verify_api_key(&api_key)
        .await
        .map_err(|e| OnboardingError::Database(format!("Telnyx API error: {e}")))?
    {
        return Err(OnboardingError::Database(
            "Telnyx API key rejected. Check your key at telnyx.com and ensure your account is funded.".to_string(),
        ));
    }
    println!("  \x1b[32m✓\x1b[0m API key valid");

    // Search available numbers (try to match user's area code)
    let area_code = user_phone
        .strip_prefix('+')
        .and_then(|s| s.get(..3))
        .or_else(|| user_phone.get(..3));
    println!("  Searching available phone numbers...");
    let numbers = search_available_numbers(&api_key, area_code)
        .await
        .map_err(|e| OnboardingError::Database(format!("Number search failed: {e}")))?;

    if numbers.is_empty() {
        return Err(OnboardingError::Database(
            "No available phone numbers found. Try again later or check your Telnyx account."
                .to_string(),
        ));
    }

    // Pick the first available number (user can refine later)
    let chosen_number = numbers[0].clone();
    println!(
        "  \x1b[32m✓\x1b[0m Found number: \x1b[36m{}\x1b[0m",
        chosen_number
    );

    // Get or create a messaging profile
    println!("  Setting up messaging profile...");
    let profile_id = get_messaging_profile(&api_key)
        .await
        .map_err(|e| OnboardingError::Database(format!("Messaging profile error: {e}")))?;
    println!("  \x1b[32m✓\x1b[0m Messaging profile ready");

    // Order the number
    println!("  Ordering phone number...");
    let ordered = order_number(&api_key, &chosen_number, &profile_id)
        .await
        .map_err(|e| OnboardingError::Database(format!("Number order failed: {e}")))?;
    println!(
        "  \x1b[32m✓\x1b[0m Number ordered: \x1b[36m{}\x1b[0m",
        ordered
    );

    // Send welcome SMS
    println!("  Sending welcome message to your phone...");
    send_welcome_sms(&api_key, &ordered, user_phone, replicant_name)
        .await
        .map_err(|e| OnboardingError::Database(format!("Welcome SMS failed: {e}")))?;
    println!(
        "  \x1b[32m✓\x1b[0m Welcome SMS sent to \x1b[36m{}\x1b[0m",
        user_phone
    );
    println!();
    println!("  \x1b[1;32m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
    println!(
        "  \x1b[1;32m  ✓  {} is now reachable!\x1b[0m",
        replicant_name
    );
    println!("  \x1b[1;32m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
    println!();
    println!("  \x1b[1mNumber:\x1b[0m     \x1b[36m{}\x1b[0m", ordered);
    println!("  \x1b[1mSMS:\x1b[0m        \x1b[32mReady\x1b[0m");
    println!("  \x1b[1mWhatsApp:\x1b[0m   \x1b[32mReady\x1b[0m");
    println!(
        "  \x1b[1mContact:\x1b[0m    Check your phone — {} just texted you!",
        replicant_name
    );
    println!();

    Ok((ordered.clone(), ordered))
}

/// Let the user select a model during onboarding.
///
/// Fetches available models. If models are found, shows a numbered
/// list and lets the user pick. If no providers are reachable, falls back to a
/// free-text prompt with a sensible default.
async fn select_model() -> String {
    let default_model = "deepseek-v4-pro".to_string();
    let models = fetch_models_for_onboarding().await;

    if models.is_empty() {
        println!("  \x1b[2m(No providers reachable — using default model)\x1b[0m");
        let input = prompt_line(&format!(
            "  Model name (default: \x1b[36m{}\x1b[0m):",
            default_model
        ));
        match input {
            Ok(s) if s.trim().is_empty() => default_model,
            Ok(s) => s.trim().to_string(),
            Err(_) => default_model,
        }
    } else {
        println!("  \x1b[1mAvailable models:\x1b[0m");
        for (i, m) in models.iter().enumerate() {
            let size_str = m
                .size_bytes
                .map(|s| format!("{:.1}GB", s as f64 / 1_073_741_824.0))
                .unwrap_or_else(|| "?".to_string());
            let family = m.family.as_deref().unwrap_or("-");
            let params = m.parameter_size.as_deref().unwrap_or("-");
            println!(
                "    {}. \x1b[36m{}\x1b[0m  {:<10} {:<8} {}",
                i + 1,
                m.name,
                family,
                params,
                size_str
            );
        }
        println!("    {}. Enter a model name manually", models.len() + 1);

        let choice = prompt_choice(
            &format!(
                "  Select a model (1-{}, default: \x1b[36m{}\x1b[0m):",
                models.len() + 1,
                default_model
            ),
            1..=(models.len() + 1),
        );

        match choice {
            Ok(n) if n <= models.len() => models[n - 1].name.clone(),
            Ok(_) => {
                // User chose "enter manually"
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
}

/// Print a summary after successful replicant creation (first-run).
fn print_creation_summary(name: &str, description: &str, model: &str) {
    println!();
    println!("  \x1b[1;32m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
    println!("  \x1b[1;32m  ✓  Replicant created successfully!\x1b[0m");
    println!("  \x1b[1;32m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
    println!();
    println!("  \x1b[1mReplicant:\x1b[0m  \x1b[36m{}\x1b[0m", name);
    println!("  \x1b[1mPurpose:\x1b[0m   {}", description);
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
