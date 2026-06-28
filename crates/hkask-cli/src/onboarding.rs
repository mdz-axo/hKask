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

use hkask_inference::{FusionConfig, FusionMode, InferenceRouter, ProviderId};
use hkask_services::{
    InferenceConfig, MatrixRegistrationResult, OnboardingService, ResolvedSecrets, ServiceConfig,
    ServiceError,
};
use hkask_storage::{RegisteredAgent, UserProfile};
use thiserror::Error;

use crate::repl::display;

mod discovery;
pub(crate) mod ui;
pub(crate) use ui::read_line;
use ui::{prompt_choice, prompt_line, prompt_passphrase, prompt_passphrase_with_confirm};

#[derive(Error, Debug)]
pub enum OnboardingError {
    #[error("Onboarding cancelled by user")]
    Cancelled,
    #[error(transparent)]
    Service(#[from] hkask_services::ServiceError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
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

            // ── Matrix pending-recovery: if Matrix registration was deferred
            //     (Conduit was down during onboarding), retry now. ──
            retry_pending_matrix(&handle).await;

            // Ensure the agent's directory space exists on disk.
            // This covers migration from old layouts where agent folders
            // may not have been created yet.
            let _ = hkask_types::agent_paths::ensure_agent_dirs(&agent_name);

            return Ok(OnboardingOutcome {
                signed_in_agent: agent_name,
                resolved_secrets: None,
                selected_model: None,
                is_first_run: false,
            });
        }
    }

    // Setup: create the user's first replicant.
    display::print_onboarding_banner();
    create_first_replicant_flow().await
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
        OnboardingError::Service(ServiceError::Config {
            source: None,
            message: "No keychain secrets — run `kask chat` first".into(),
        })
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
    let name = name.trim().to_string();
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
    setup_provider().await?;
    let selected_model = select_model().await?;

    OnboardingService::register_replicant(
        &handle.a2a,
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

    // Create the agent's directory space immediately — don't wait for first
    // pod deployment. The agent folder is their digital sphere: sessions,
    // memory, artifacts, and pod storage all live here.
    if let Err(e) = hkask_types::agent_paths::ensure_agent_dirs(&display_name) {
        eprintln!(
            "  \x1b[33m⚠\x1b[0m  Could not create agent directory: {}",
            e
        );
    }

    // Matrix registration for the new replicant (human account already exists).
    // Recovery logic lives in the service layer.
    let homeserver_url =
        std::env::var("HKASK_MATRIX_URL").unwrap_or_else(|_| "http://localhost:8008".to_string());
    let matrix_info =
        match OnboardingService::register_replicant_matrix_account(&display_name, &homeserver_url)
            .await
        {
            Ok(user_id) => Some(user_id),
            Err(e) => {
                eprintln!("  \x1b[33m⚠\x1b[0m  Matrix registration failed: {}", e);
                None
            }
        };

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
    if let Some(ref mid) = matrix_info {
        println!("  \x1b[1mMatrix:\x1b[0m    \x1b[36m{}\x1b[0m", mid);
    }
    println!();
    println!(
        "  Start a session: \x1b[36mkask chat {}\x1b[0m",
        display_name
    );
    println!();

    Ok(())
}

// ── Private helpers ────────────────────────────────────────────────────────

/// Select which replicant to sign into when multiple exist.
fn select_replicant(replicants: &[RegisteredAgent]) -> Result<String, OnboardingError> {
    println!("\n  \x1b[1mRegistered replicants:\x1b[0m");
    for (i, r) in replicants.iter().enumerate() {
        let desc = r
            .definition
            .charter
            .as_ref()
            .map(|c| c.purpose.as_str())
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

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  user must not cancel at any interactive prompt
/// post: returns OnboardingOutcome with signed_in_agent, resolved_secrets, selected_model, is_first_run=true; all secrets derived and stored in keychain; replicant registered in A2A; user profile stored; matrix registration attempted (non-blocking)
/// inv:  does not modify any external state before derive_secrets; cancellation at any prompt returns OnboardingError::Cancelled with zero side effects
/// Flow: Create the user's first replicant
async fn create_first_replicant_flow() -> Result<OnboardingOutcome, OnboardingError> {
    println!("\n  \x1b[1mWelcome to hKask!\x1b[0m");
    println!("  Let's set up your profile and your first replicant.\n");

    // ── Human identity (collected first — replicant naming depends on it) ──

    // Q1: Human first name
    let human_first = prompt_line("  What is your first name?")?;

    // Q2: Human last name
    println!();
    let human_last = prompt_line("  What is your last name?")?;

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

    println!();
    let name = prompt_line("  What first name should your replicant have?")?;
    let name = name.trim().to_string();
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

    // ── Interactive prompts (CLI layer, not the state machine) ──

    // Provider setup
    println!();
    setup_provider().await?;

    // Model selection
    println!();
    println!("  \x1b[1mChoose a model\x1b[0m for your replicant to use.");
    println!("  Models determine how your replicant thinks and responds.");
    let selected_model = select_model().await?;

    // Passphrase
    println!();
    println!("  Choose a \x1b[1mmaster passphrase\x1b[0m to encrypt your data.");
    println!("  This passphrase derives all your internal security keys.");
    println!("  \x1b[2mStore it in a password manager — it cannot be recovered if lost.\x1b[0m");
    let passphrase = prompt_passphrase_with_confirm()?;

    // ── Run the state machine for all service calls ──
    use crate::onboarding_session::OnboardingSession;
    let session = OnboardingSession::new(user_profile, name, description);
    let completed = session
        .run(|| Ok(selected_model.clone()), || Ok(passphrase.clone()))
        .await
        .map_err(|(_session, e)| e)?;

    // Post-creation summary
    print_creation_summary(
        &completed.display_name,
        &completed.description,
        &completed.selected_model,
        completed.matrix_result.as_ref(),
    );

    // Create the agent's directory space on disk.
    let _ = hkask_types::agent_paths::ensure_agent_dirs(&completed.display_name);

    Ok(OnboardingOutcome {
        signed_in_agent: completed.display_name,
        resolved_secrets: completed.resolved_secrets,
        selected_model: Some(completed.selected_model),
        is_first_run: true,
    })
}


/// Resolve the display name for the currently active provider.
fn provider_display_name(config: &InferenceConfig) -> &'static str {
    match config.default_provider {
        ProviderId::KiloCode => "KiloCode",
        ProviderId::DeepInfra => "DeepInfra",
        ProviderId::Together => "Together AI",
        ProviderId::Fal => "fal.ai",
        ProviderId::OpenRouter => "OpenRouter",
        _ => "your provider",
    }
}

/// Let the user select a model using the dynamic discovery pipeline.
async fn select_model() -> Result<String, OnboardingError> {
    let config = InferenceConfig::from_env();
    let default_model = config.default_model.clone();
    let provider_name = provider_display_name(&config);

    // Run the discovery pipeline (HF → classify → dedup → fallback)
    let (models, source_label) = discovery::discover_models(&config).await;
    let is_dynamic = models.first().map(|m| m.source == discovery::ModelSource::Dynamic).unwrap_or(false);

    let display_source = if is_dynamic {
        format!("via {}", source_label)
    } else {
        source_label
    };

    println!("  \x1b[1mAvailable models\x1b[0m ({}):", display_source);
    println!();

    // Group by family for structured display
    let mut families: Vec<String> = models.iter().map(|m| m.family.clone()).collect();
    families.sort();
    families.dedup();

    let mut idx = 1usize;
    for family in &families {
        let family_models: Vec<&discovery::OnboardingModel> = models
            .iter()
            .filter(|m| &m.family == family)
            .collect();
        if family_models.is_empty() { continue; }

        println!("  \x1b[1m{}\x1b[0m", discovery::shorten_for_display(family));
        for m in &family_models {
            let marker = if m.full_id == default_model {
                " \x1b[33m(default)\x1b[0m"
            } else {
                ""
            };
            let kind_icon = match m.kind {
                discovery::ModelKind::Thinking => "\x1b[35m🧠\x1b[0m",
                discovery::ModelKind::Instruct => "\x1b[32m📋\x1b[0m",
            };
            println!(
                "    {}. {} \x1b[36m{}\x1b[0m{}  \x1b[2m{}\x1b[0m",
                idx, kind_icon, m.label, marker, m.description
            );
            idx += 1;
        }
        println!();
    }

    // Offer hKask fusion (our own orchestrator, provider-agnostic)
    let fusion_configured = config.fusion.is_some();
    if fusion_configured {
        let fusion = config
            .fusion
            .as_ref()
            .map(|f| {
                let mode = match f.mode {
                    FusionMode::BestOfN => "Best-of-N",
                    FusionMode::Synthesis => "Synthesis",
                    FusionMode::Critique => "Critique",
                    FusionMode::Deliberation => "Deliberation",
                    FusionMode::PlanImplement => "Plan/Implement",
                };
                format!("⚡ Fusion [{}] — {}", mode, f.description())
            })
            .unwrap_or_else(|| "⚡ Fusion (kask defaults)".to_string());
        println!("    {}. \x1b[1;33m{}\x1b[0m", idx, fusion);
        idx += 1;
        println!();
    }

    let manual_idx = idx;
    println!("    {}. Enter a model name manually", manual_idx);
    println!();

    let choice = prompt_choice(
        &format!(
            "  Select a model (1-{}, default: \x1b[36m{}\x1b[0m):",
            manual_idx, default_model
        ),
        1..=(manual_idx),
    );

    let fusion_idx = models.len() + 1;
    let model_count = models.len();
    match choice {
        Ok(n) if n <= model_count => Ok(models[n - 1].full_id.clone()),
        Ok(n) if fusion_configured && n == fusion_idx => Ok(config
            .fusion
            .as_ref()
            .map(|f| f.model_id())
            .unwrap_or_else(|| "fusion/default".to_string())),
        Ok(_) => {
            let input = prompt_line("  Model name:")?;
            if input.trim().is_empty() {
                Ok(default_model)
            } else {
                Ok(input.trim().to_string())
            }
        }
        Err(e) => Err(e),
    }
}

/// Interactive provider setup during first-run onboarding.
///
/// Checks if a provider API key is already configured (env var or keychain —
/// .env is auto-loaded by dotenvy at startup). If not, prompts to enter a key
/// directly or skip.
async fn setup_provider() -> Result<(), OnboardingError> {
    let config = InferenceConfig::from_env();

    // Check if any cloud provider is already configured
    let has_deepinfra = !config.deepinfra_api_key.is_empty();
    let has_together = !config.together_api_key.is_empty();
    let has_fal = !config.fal_api_key.is_empty();
    let has_kilocode = !config.kilocode_api_key.is_empty();

    if has_deepinfra || has_together || has_fal || has_kilocode {
        let provider_name = if has_kilocode {
            "KiloCode"
        } else if has_deepinfra {
            "DeepInfra"
        } else if has_together {
            "Together AI"
        } else {
            "fal.ai"
        };
        println!(
            "  \x1b[32m✓\x1b[0m {} API key found — using {} as default provider.",
            provider_name, provider_name
        );
        // Auto-load into keychain so future sessions don't need .env in cwd
        let keychain = hkask_keystore::Keychain::default();
        if has_kilocode {
            let _ = keychain.store_by_key("KILOCODE_API_KEY", &config.kilocode_api_key);
        }
        if has_deepinfra {
            let _ = keychain.store_by_key("DI_API_KEY", &config.deepinfra_api_key);
        }
        if has_together {
            let _ = keychain.store_by_key("TOGETHER_API_KEY", &config.together_api_key);
        }
        if has_fal {
            let _ = keychain.store_by_key("FA_API_KEY", &config.fal_api_key);
        }
        return Ok(());
    }

    // No cloud provider configured — prompt the user
    println!("  \x1b[1mInference provider\x1b[0m");
    println!();
    println!("  hKask requires an inference provider to generate responses.");
    println!("  Without one, your replicant cannot reply to you.");
    println!();
    println!("  An API key is like a password that lets hKask use a cloud");
    println!("  AI service. You can get a free key at:");
    println!();
    println!(
        "    \x1b[36mhttps://kilo.ai/\x1b[0m         (KiloCode — unified gateway, recommended)"
    );
    println!("    \x1b[36mhttps://deepinfra.com/\x1b[0m  (free tier, wide model catalog)");
    println!("    \x1b[36mhttps://together.ai/\x1b[0m  (inference + fine-tuning)");
    println!("    \x1b[36mhttps://fal.ai/\x1b[0m      (specialized vision/OCR models)");
    println!();
    println!("  Set your key in .env (auto-loaded at startup) or enter it now.");
    println!();
    println!("    1. Enter API key directly (input is hidden)");
    println!("    2. Skip for now");
    println!();

    let choice = prompt_choice("  Choice (1-2):", 1..=2)?;

    match choice {
        1 => {
            // Enter API key directly
            println!();
            println!("  Supported providers:");
            println!("    KC — KiloCode (unified gateway, recommended)");
            println!("    DI — DeepInfra (wide model catalog)");
            println!("    TG — Together AI (inference + fine-tuning)");
            println!("    FA — fal.ai (specialized vision/OCR models)");
            println!();

            let provider_str = prompt_line("  Provider code (KC/DI/TG/FA):")?;
            let provider_str = provider_str.trim().to_uppercase();

            let key_name = match provider_str.as_str() {
                "KC" => "KILOCODE_API_KEY",
                "DI" => "DI_API_KEY",
                "TG" => "TOGETHER_API_KEY",
                "FA" => "FA_API_KEY",
                _ => {
                    println!(
                        "  \x1b[31m✗\x1b[0m Unknown provider '{}'. Use KC, DI, TG, or FA.",
                        provider_str
                    );
                    return Err(OnboardingError::Cancelled);
                }
            };

            let api_key = prompt_passphrase(&format!("  {} API key:", key_name))?;
            let api_key = api_key.trim();
            if api_key.is_empty() {
                println!("  No key entered — skipping provider setup.");
                return Ok(());
            }

            let keychain = hkask_keystore::Keychain::default();
            keychain.store_by_key(key_name, api_key).map_err(|e| {
                eprintln!("  \x1b[31m✗\x1b[0m Failed to store key: {}", e);
                OnboardingError::Service(ServiceError::Keystore {
                    source: Some(Box::new(e)),
                    message: format!("Failed to store {}", key_name),
                })
            })?;

            // Also set default provider to match
            let _ = keychain.store_by_key(
                hkask_types::keychain_keys::KEY_DEFAULT_PROVIDER,
                &provider_str,
            );

            println!("  \x1b[32m✓\x1b[0m Key stored in OS keychain.");
            println!("  Default provider set to {}", provider_str);
        }
        2 => {
            println!();
            println!("  \x1b[33m⚠\x1b[0m  Skipping cloud provider setup.");
            println!("  hKask requires a cloud inference provider to generate responses.");
            println!("  Without one, your replicant cannot reply to you.");
            println!();
            println!("  To add a cloud provider later, add your key to .env and restart.");
        }
        _ => unreachable!(),
    }

    Ok(())
}

// ── Private helpers ────────────────────────────────────────────────────────

/// Retry pending Matrix registration silently on session start.
async fn retry_pending_matrix(handle: &hkask_services::RegistryHandle) {
    let keychain = hkask_keystore::Keychain::default();
    if keychain
        .retrieve_by_key(hkask_types::keychain_keys::KEY_MATRIX_PENDING_RECOVERY)
        .unwrap_or_default()
        != "true"
    {
        return;
    }
    // Already registered? Clear the marker.
    if keychain
        .retrieve_by_key(hkask_types::keychain_keys::KEY_MATRIX_REPLICANT_USERNAME)
        .is_ok()
    {
        let _ = keychain.delete_by_key(hkask_types::keychain_keys::KEY_MATRIX_PENDING_RECOVERY);
        return;
    }
    // Load what we need and delegate to the service (which handles recovery).
    let homeserver_url = keychain
        .retrieve_by_key(hkask_types::keychain_keys::KEY_MATRIX_PENDING_HOMESERVER)
        .unwrap_or_else(|_| "http://localhost:8008".to_string());
    let user_profile = match hkask_services::OnboardingService::get_user_profile(&handle.store) {
        Ok(Some(p)) => p,
        _ => return,
    };
    let replicants = match list_replicants(&handle.store) {
        Ok(r) if !r.is_empty() => r,
        _ => return,
    };
    let replicant_name = replicants[0].definition.name.clone();
    let passphrase =
        match keychain.retrieve_by_key(hkask_types::keychain_keys::KEY_MASTER_PASSPHRASE) {
            Ok(p) => p,
            _ => return,
        };
    let _ = hkask_services::OnboardingService::register_matrix_accounts(
        &user_profile,
        &replicant_name,
        &passphrase,
        &homeserver_url,
    )
    .await;
}

/// Print a summary after successful replicant creation (first-run).
fn print_creation_summary(
    name: &str,
    description: &str,
    model: &str,
    matrix: Option<&MatrixRegistrationResult>,
) {
    println!();
    println!("  \x1b[1;32m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
    println!("  \x1b[1;32m  ✓  Replicant created successfully!\x1b[0m");
    println!("  \x1b[1;32m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
    println!();
    println!("  \x1b[1mReplicant:\x1b[0m  \x1b[36m{}\x1b[0m", name);
    println!("  \x1b[1mTag line:\x1b[0m  {}", description);
    println!("  \x1b[1mModel:\x1b[0m     \x1b[36m{}\x1b[0m", model);
    println!("  \x1b[1mSecurity:\x1b[0m  Keys stored in OS keychain (encrypted DB)");

    if let Some(m) = matrix {
        println!();
        println!("  \x1b[1mMatrix Chat:\x1b[0m");
        println!("  Accounts registered on Conduit (localhost:8008):");
        println!("    \x1b[36mYou:\x1b[0m      {}", m.human_user_id);
        println!("    \x1b[36mReplicant:\x1b[0m {}", m.replicant_user_id);
        println!();
        println!("  Open FluffyChat (or any Matrix client) and log in with:");
        println!("    Homeserver: http://localhost:8008");
        println!("    Username:   {}", m.human_user_id);
        println!("    Password:   your master passphrase");
    }

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
        .map_err(|e| {
            OnboardingError::Service(ServiceError::AgentRegistryStore {
                message: e.to_string(),
            })
        })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_display_kilocode() {
        let mut config = InferenceConfig::default();
        config.default_provider = ProviderId::KiloCode;
        assert_eq!(provider_display_name(&config), "KiloCode");
    }

    #[test]
    fn provider_display_deepinfra() {
        let config = InferenceConfig::default();
        assert_eq!(provider_display_name(&config), "DeepInfra");
    }
}
