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

use hkask_inference::{InferenceRouter, ProviderId};
use hkask_services::{
    InferenceConfig, MatrixRegistrationResult, OnboardingService, ResolvedSecrets, ServiceConfig,
    ServiceError,
};
use hkask_storage::{RegisteredAgent, UserProfile};
use thiserror::Error;

use crate::repl::display;

mod ui;
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

/// Keywords that identify non-LLM models to exclude from onboarding.
/// Models matching these are embeddings, TTS, OCR, image/video gen, etc.
const NON_LLM_KEYWORDS: &[&str] = &[
    "embedding",
    "bge-",
    "gte-",
    "e5-",
    "stella",
    "jina-embeddings",
    "tts",
    "speech",
    "whisper",
    "parakeet",
    "zonos",
    "chatterbox",
    "ocr",
    "document-",
    "layout",
    "paddleocr",
    "got-ocr",
    "flux",
    "stable-diffusion",
    "sd-",
    "sdxl",
    "imagen",
    "dall-e",
    "image",
    "video",
    "vision",
    "clip",
    "blip",
    "reranker",
    "bge-reranker",
    "jina-reranker",
    "voice",
    "audio",
    "melotts",
    "fish-speech",
    "upscale",
    "segmentation",
    "detection",
    "depth",
    "bgm",
    "musicgen",
];

/// Keywords that indicate a model is small / not frontier-tier.
/// Models matching these are excluded from the near-frontier list.
const SMALL_MODEL_KEYWORDS: &[&str] = &[
    "-1b",
    "-3b",
    "-7b",
    "-8b",
    "-0.5b",
    "-1.5b",
    "-0.6b",
    "-mini",
    "-tiny",
    "-small",
    "-nano",
    "instruct-1b",
    "instruct-3b",
    "instruct-7b",
    "instruct-8b",
];

/// Keywords that signal a model is likely frontier-tier.
const FRONTIER_SIGNALS: &[(&str, u32)] = &[
    ("pro", 20),
    ("max", 18),
    ("ultra", 18),
    ("super", 15),
    ("flash", 8),
    ("large", 10),
    ("preview", 5),
];

/// Static fallback model list — used ONLY when provider APIs are unreachable.
/// These are never shown when dynamic discovery succeeds.
const ONBOARDING_FALLBACK_MODELS: &[(&str, &str)] = &[
    (
        "DI/deepseek-ai/DeepSeek-V4-Pro",
        "1.6T MoE, 1M ctx, top coding/reasoning",
    ),
    (
        "DI/zai-org/GLM-5.2",
        "744B MoE, MIT, leads open-weight GPQA",
    ),
    (
        "DI/moonshotai/Kimi-K2.6",
        "1T MoE, agent swarms, multimodal",
    ),
    (
        "DI/MiniMaxAI/MiniMax-M3",
        "1M ctx, multimodal, top SWE-Bench",
    ),
    (
        "DI/Qwen/Qwen3.5-397B-A17B",
        "397B MoE, 262K ctx, Apache 2.0",
    ),
    (
        "DI/nvidia/NVIDIA-Nemotron-3-Super-120B-A12B",
        "120B MoE, 1M ctx",
    ),
    (
        "DI/google/gemma-4-31B-it",
        "31B dense, Apache 2.0, strong reasoning",
    ),
    (
        "DI/deepseek-ai/DeepSeek-V4-Flash",
        "284B MoE, efficient 1M ctx",
    ),
];

/// Result of dynamic model discovery from configured providers.
struct OnboardingModel {
    label: String,
    full_id: String,
    description: String,
    provider: ProviderId,
    score: u32,
}

/// Fetch available models from configured cloud providers.
///
/// Queries each configured provider's model listing API (which already
/// filters to models updated in the last 180 days for DeepInfra & KiloCode).
/// Then applies dynamic heuristics to identify near-frontier open-weight LLMs:
///
/// 1. Exclude non-LLM models (embeddings, TTS, OCR, image gen, etc.)
/// 2. Exclude very small models (< ~10B parameters)
/// 3. Score remaining models by size signals and quality tier indicators
/// 4. Return top-scoring models sorted by quality
///
/// Falls back to a static curated list only when no providers are reachable.
async fn fetch_onboarding_models(config: &InferenceConfig) -> Vec<OnboardingModel> {
    let router = InferenceRouter::new(config.clone());
    let all_models = router.list_models().await;

    if !all_models.is_empty() {
        let mut scored: Vec<OnboardingModel> = all_models
            .into_iter()
            .filter(|m| is_likely_llm(&m.model))
            .filter(|m| !is_likely_small_model(&m.model))
            .map(|m| {
                let score = compute_frontier_score(&m.model);
                let desc = describe_model_dynamic(&m.model);
                OnboardingModel {
                    label: shorten_model_id(&m.model),
                    full_id: m.prefixed_name,
                    description: desc,
                    provider: m.provider,
                    score,
                }
            })
            .collect();

        // Sort by score descending, then alphabetically for ties
        scored.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.label.cmp(&b.label)));

        // Deduplicate by full_id
        let mut seen = std::collections::HashSet::new();
        scored.retain(|m| seen.insert(m.full_id.clone()));

        // Show top 12 to avoid overwhelming the user
        scored.truncate(12);

        if !scored.is_empty() {
            return scored;
        }
    }

    // Fallback: static curated list (API unreachable)
    ONBOARDING_FALLBACK_MODELS
        .iter()
        .map(|(id, desc)| OnboardingModel {
            label: shorten_model_id(id),
            full_id: id.to_string(),
            description: desc.to_string(),
            provider: ProviderId::DeepInfra,
            score: 0,
        })
        .collect()
}

/// Check if a model name suggests it's a text-generation LLM.
fn is_likely_llm(model_id: &str) -> bool {
    let lower = model_id.to_lowercase();
    !NON_LLM_KEYWORDS.iter().any(|kw| lower.contains(kw))
}

/// Check if a model name suggests it's small (< ~10B parameters).
fn is_likely_small_model(model_id: &str) -> bool {
    let lower = model_id.to_lowercase();
    SMALL_MODEL_KEYWORDS.iter().any(|kw| lower.contains(kw))
}

/// Compute a dynamic frontier quality score from the model name alone.
///
/// Scoring signals (all extracted dynamically, no hardcoded model names):
/// - Parameter count in the name (e.g., "397B" → +39, "120B" → +12)
/// - Quality tier keywords ("Pro" → +20, "Max" → +18, etc.)
/// - Version numbers (e.g., "V4" → +4, "3.5" → +3)
fn compute_frontier_score(model_id: &str) -> u32 {
    let lower = model_id.to_lowercase();
    let mut score: u32 = 10;

    score += extract_param_score(&lower);

    for (signal, points) in FRONTIER_SIGNALS {
        if lower.contains(signal) {
            score += points;
        }
    }

    score += extract_version_score(&lower);

    score
}

/// Extract a score from parameter count patterns in the model name.
fn extract_param_score(lower: &str) -> u32 {
    let mut best: u32 = 0;

    for cap in lower.split(|c: char| !c.is_alphanumeric() && c != '.') {
        if cap.ends_with('t') {
            if let Ok(val) = cap.trim_end_matches('t').parse::<f64>() {
                best = best.max((val * 100.0) as u32);
            }
        }
        if cap.ends_with('b') {
            if let Ok(val) = cap.trim_end_matches('b').parse::<f64>() {
                best = best.max((val / 10.0) as u32);
            }
        }
    }

    best.min(200)
}

/// Extract a score from version number patterns.
fn extract_version_score(lower: &str) -> u32 {
    let mut score: u32 = 0;

    for word in lower.split(|c: char| !c.is_alphanumeric()) {
        if word.len() >= 2 && word.starts_with('v') {
            if let Ok(val) = word[1..].parse::<u32>() {
                score += val.min(10);
            }
        }
    }

    for part in lower.split(|c: char| c.is_whitespace() || c == '-' || c == '/') {
        let bytes = part.as_bytes();
        if bytes.len() >= 3
            && bytes[0].is_ascii_digit()
            && bytes[1] == b'.'
            && bytes[2].is_ascii_digit()
        {
            let major = (bytes[0] - b'0') as u32;
            score += major.min(10);
        }
    }

    score
}

/// Shorten a model ID for display.
/// "deepseek-ai/DeepSeek-V4-Pro" → "DeepSeek V4 Pro"
fn shorten_model_id(id: &str) -> String {
    let base = id.splitn(2, '/').nth(1).unwrap_or(id);
    let base = base.splitn(2, '/').nth(1).unwrap_or(base);
    let base = if base.is_empty() { id } else { base };

    base.replace('-', " ")
        .replace('_', " ")
        .split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Generate a concise description for a model — fully dynamic.
fn describe_model_dynamic(model_id: &str) -> String {
    let lower = model_id.to_lowercase();
    let mut parts: Vec<String> = Vec::new();

    let param_str = extract_param_display(&lower);
    if !param_str.is_empty() {
        parts.push(param_str);
    }

    if extract_moe_active(&lower).is_some() {
        parts.push("MoE".into());
    }

    for (signal, _) in FRONTIER_SIGNALS {
        if lower.contains(signal) {
            let tier = match *signal {
                "pro" => "Pro tier",
                "max" => "Max tier",
                "ultra" => "Ultra tier",
                "super" => "Super tier",
                "flash" => "Flash (efficient)",
                "large" => "Large",
                _ => "",
            };
            if !tier.is_empty() && !parts.iter().any(|p| p.contains("tier")) {
                parts.push(tier.into());
                break;
            }
        }
    }

    if lower.contains("coder") || lower.contains("code") {
        parts.push("coding-specialized".into());
    }
    if lower.contains("reasoning") || lower.contains("think") {
        parts.push("reasoning".into());
    }
    if lower.contains("instruct") || lower.contains("-it") {
        if !parts.iter().any(|p| p.contains("instruction")) {
            parts.push("instruction-tuned".into());
        }
    }
    if lower.contains("multimodal") || lower.contains("omni") {
        parts.push("multimodal".into());
    }

    if parts.is_empty() {
        "Open-weight LLM (recently updated)".into()
    } else {
        parts.join(", ")
    }
}

/// Extract a human-readable parameter count from model name.
fn extract_param_display(lower: &str) -> String {
    let mut best: String = String::new();
    let mut best_val: f64 = 0.0;

    for cap in lower.split(|c: char| !c.is_alphanumeric() && c != '.') {
        if cap.ends_with('t') {
            if let Ok(val) = cap.trim_end_matches('t').parse::<f64>() {
                if val > best_val {
                    best_val = val;
                    best = format!("{}T", val);
                }
            }
        }
        if cap.ends_with('b') {
            if let Ok(val) = cap.trim_end_matches('b').parse::<f64>() {
                let b_val = val / 1000.0;
                if b_val > best_val {
                    best_val = b_val;
                    best = format!("{}B", val);
                }
            }
        }
    }

    best
}

/// Extract MoE active parameter count if model name suggests MoE architecture.
fn extract_moe_active(lower: &str) -> Option<u32> {
    for part in lower.split(|c: char| !c.is_alphanumeric()) {
        if part.len() >= 3 && part.starts_with('a') && part.ends_with('b') {
            if let Ok(val) = part[1..part.len() - 1].parse::<u32>() {
                return Some(val);
            }
        }
    }
    None
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

/// Let the user select a model — dynamically discovered from providers or fallback.
async fn select_model() -> Result<String, OnboardingError> {
    let config = InferenceConfig::from_env();
    let default_model = config.default_model.clone();
    let provider_name = provider_display_name(&config);

    // Dynamically discover models from configured providers
    let models = fetch_onboarding_models(&config).await;
    let is_dynamic = !models.is_empty() && models[0].score > 0;

    let source_label = if is_dynamic {
        format!("dynamically discovered via {} (≤6 months)", provider_name)
    } else {
        format!("curated fallback list ({} unreachable)", provider_name)
    };

    println!("  \x1b[1mAvailable models\x1b[0m ({}):", source_label);
    println!();

    let mut idx = 1usize;
    for m in &models {
        let marker = if m.full_id == default_model || m.label == default_model {
            " \x1b[33m(default)\x1b[0m"
        } else {
            ""
        };
        println!(
            "    {}. \x1b[36m{}\x1b[0m{}  \x1b[2m{}\x1b[0m",
            idx, m.label, marker, m.description
        );
        idx += 1;
    }

    // Offer fusion if OpenRouter is configured
    let has_openrouter = !config.openrouter_api_key.is_empty();
    if has_openrouter {
        let fusion = config
            .fusion
            .as_ref()
            .map(|f| format!("openrouter/fusion ({})", f.description()))
            .unwrap_or_else(|| "openrouter/fusion (kask defaults)".to_string());
        println!();
        println!(
            "    {}. \x1b[1;33m⚡ Fusion: \x1b[36m{}\x1b[0m\x1b[0m",
            idx, fusion
        );
        idx += 1;
    }
    let manual_idx = idx;
    println!();
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
        Ok(n) if has_openrouter && n == fusion_idx => Ok(config
            .fusion
            .as_ref()
            .map(|f| f.model_id())
            .unwrap_or_else(|| "openrouter/fusion".to_string())),
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

    // ── is_likely_llm ─────────────────────────────────────────────────────

    #[test]
    fn llm_filter_passes_text_models() {
        assert!(is_likely_llm("deepseek-ai/DeepSeek-V4-Pro"));
        assert!(is_likely_llm("deepseek-v4-pro"));
        assert!(is_likely_llm("zai-org/GLM-5.1"));
        assert!(is_likely_llm("moonshotai/Kimi-K2.6"));
    }

    #[test]
    fn llm_filter_rejects_embeddings() {
        assert!(!is_likely_llm("BAAI/bge-m3"));
        assert!(!is_likely_llm("intfloat/e5-mistral-7b-instruct"));
    }

    #[test]
    fn llm_filter_rejects_image_gen() {
        assert!(!is_likely_llm("black-forest-labs/FLUX.1-dev"));
        assert!(!is_likely_llm("stabilityai/stable-diffusion-3.5-large"));
    }

    #[test]
    fn llm_filter_passes_vision_language_models() {
        assert!(is_likely_llm("qwen-vl-max"));
        assert!(is_likely_llm("llava-13b"));
    }

    // ── is_likely_small_model ─────────────────────────────────────────────

    #[test]
    fn small_filter_rejects_tiny_models() {
        assert!(is_likely_small_model("qwen3-8b"));
        assert!(is_likely_small_model("llama-3b-instruct"));
        assert!(is_likely_small_model("phi-3-mini"));
    }

    #[test]
    fn small_filter_passes_large_models() {
        assert!(!is_likely_small_model("deepseek-ai/DeepSeek-V4-Pro"));
        assert!(!is_likely_small_model("Qwen/Qwen3.5-397B-A17B"));
        assert!(!is_likely_small_model("google/gemma-4-31B-it"));
    }

    // ── parse_params ──────────────────────────────────────────────────────

    #[test]
    fn parse_billion_params() {
        let p = parse_params("qwen3.5-397b-a17b");
        assert_eq!(p.display, "397B");
        assert_eq!(p.score, 39);
    }

    #[test]
    fn parse_trillion_params() {
        let p = parse_params("deepseek-v4-1.6t");
        assert_eq!(p.display, "1.6T");
        assert_eq!(p.score, 160);
    }

    #[test]
    fn parse_no_params() {
        let p = parse_params("claude-sonnet");
        assert!(p.display.is_empty());
        assert_eq!(p.score, 0);
    }

    // ── compute_frontier_score ────────────────────────────────────────────

    #[test]
    fn frontier_score_ranks_pro_over_flash() {
        let pro = compute_frontier_score("deepseek-v4-pro", 160);
        let flash = compute_frontier_score("deepseek-v4-flash", 28);
        assert!(pro > flash, "Pro ({pro}) should outrank Flash ({flash})");
    }

    #[test]
    fn frontier_score_ranks_large_over_small() {
        let large = compute_frontier_score("qwen3.5-397b-a17b", 39);
        let small = compute_frontier_score("qwen3-70b", 7);
        assert!(large > small);
    }

    // ── shorten_model_id ──────────────────────────────────────────────────

    #[test]
    fn shorten_deepinfra_model() {
        let result = shorten_model_id("DI/deepseek-ai/DeepSeek-V4-Pro");
        assert_eq!(result, "Deepseek V4 Pro");
    }

    #[test]
    fn shorten_kilocode_model() {
        let result = shorten_model_id("KC/deepseek-v4-pro");
        assert_eq!(result, "Deepseek V4 Pro");
    }

    #[test]
    fn shorten_flat_model_id() {
        let result = shorten_model_id("deepseek-v4-pro");
        assert_eq!(result, "Deepseek V4 Pro");
    }

    // ── provider_display_name ─────────────────────────────────────────────

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
