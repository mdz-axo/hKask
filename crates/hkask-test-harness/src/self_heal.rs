//! Self-Healing Engine — unified error recovery for hKask.
//!
//! Every fallible operation in the system should pass through a `SelfHealer`.
//! The healer maps error patterns to recovery strategies, attempts healing,
//! and returns either a healed state (retry), a degraded fallback (continue),
//! or an unhealable report (escalate to Curator via CNS).
//!
//! Architecture:
//! ```text
//! Error → SelfHealer::attempt(error, context)
//!   ├── HealRegistry.try_heal(pattern) → HealAction
//!   │     ├── Healed  → retry the operation
//!   │     ├── Degraded → continue with fallback
//!   │     └── Unhealable → escalate to Curator
//!   └── CNS span emitted regardless of outcome
//! ```
//!
//! Healing strategies are defined in a registry (YAML-backed, evolvable).
//! Each strategy has: error pattern, diagnostic step, healing action,
//! verification step, and fallback behavior.

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::process::Command;

// ── Core types ──────────────────────────────────────────────────────────────

/// Outcome of a healing attempt.
#[derive(Debug, Clone)]
pub enum HealOutcome {
    /// Successfully healed — the operation can be retried.
    Healed {
        /// What action was taken to heal.
        action_taken: String,
        /// Any state that was modified (for CNS audit).
        modifications: Vec<String>,
    },
    /// Could not fully heal, but a degraded fallback exists.
    Degraded {
        /// Why healing failed.
        reason: String,
        /// Fallback value or behavior to use instead.
        fallback_description: String,
    },
    /// Cannot heal — must escalate to Curator/human.
    Unhealable {
        /// Why healing is impossible.
        reason: String,
        /// Suggested action for the operator.
        suggestion: String,
        /// Whether this requires code changes (vs config/settings).
        requires_code_change: bool,
    },
}

/// Context available to healing strategies for diagnosis.
#[derive(Debug, Clone, Default)]
pub struct HealContext {
    /// The operation that failed (e.g., "classify", "read_file").
    pub operation: String,
    /// The error message.
    pub error_message: String,
    /// Environment variables available for diagnosis.
    pub env_vars: HashMap<String, String>,
    /// Paths that may contain configuration files.
    pub config_search_paths: Vec<PathBuf>,
    /// Whether the operation can be retried after healing.
    pub can_retry: bool,
}

/// A single healing strategy — pattern match → diagnostic → heal action.
#[derive(Debug, Clone)]
pub struct HealStrategy {
    /// Human-readable name for CNS reporting.
    pub name: String,
    /// Substring or regex pattern to match against error messages.
    pub error_pattern: String,
    /// Description of what this strategy does.
    pub description: String,
    /// The healing action to perform.
    pub action: HealAction,
    /// Whether this strategy can modify files.
    pub can_modify_files: bool,
}

/// What a healing strategy actually does.
#[derive(Debug, Clone)]
pub enum HealAction {
    /// Run a shell command (diagnostic or healing).
    RunCommand {
        command: String,
        /// If true, the command's stdout is used as the healed value.
        capture_output: bool,
    },
    /// Set an environment variable.
    SetEnv {
        key: String,
        /// How to derive the value (literal, from command output, from file).
        value_source: EnvValueSource,
    },
    /// Load environment variables from a file (.env, config, etc.).
    LoadDotEnv {
        /// Paths to search for .env files.
        search_paths: Vec<PathBuf>,
    },
    /// Create a default file if it doesn't exist.
    CreateDefaultFile { path: PathBuf, content: String },
    /// Retry the operation with exponential backoff.
    RetryWithBackoff { max_attempts: u32, delay_ms: u64 },
    /// Propose a code change (for Curator review).
    ProposeCodeChange {
        file: PathBuf,
        description: String,
        diff_suggestion: String,
    },
    /// Composite: try multiple heal actions in sequence.
    Sequence(Vec<HealAction>),
}

/// How to derive an environment variable value.
#[derive(Debug, Clone)]
pub enum EnvValueSource {
    /// Literal string value.
    Literal(String),
    /// Read from a file's first line.
    FromFile(PathBuf),
    /// Run a command and use its stdout.
    FromCommand(String),
    /// Try multiple sources in order.
    FirstOf(Vec<EnvValueSource>),
}

/// Result of running a heal action.
#[derive(Debug, Clone)]
struct ActionResult {
    success: bool,
    output: String,
    modifications: Vec<String>,
}

// ── Heal Registry ───────────────────────────────────────────────────────────

/// Registry of healing strategies, keyed by error pattern.
///
/// Strategies are tried in registration order. First match wins.
/// The registry is populated from code defaults and can be extended
/// via YAML configuration at runtime.
#[derive(Debug, Clone, Default)]
pub struct HealRegistry {
    strategies: Vec<HealStrategy>,
}

impl HealRegistry {
    /// Create a registry with built-in healing strategies.
    pub fn with_defaults() -> Self {
        let mut registry = Self::default();

        // ── Strategy 1: Missing API key ─────────────────────────────────
        registry.add(HealStrategy {
            name: "missing-api-key".into(),
            error_pattern: "No API key".into(),
            description: "API key not found in environment — try loading from .env files".into(),
            action: HealAction::Sequence(vec![
                // Step 1: Try loading .env from common locations
                HealAction::LoadDotEnv {
                    search_paths: vec![
                        PathBuf::from(".env"),
                        PathBuf::from("../.env"),
                        PathBuf::from("../../.env"),
                        dirs::home_dir()
                            .unwrap_or_default()
                            .join(".config/hkask/.env"),
                    ],
                },
                // Step 2: If still missing, try alternate env var names
                HealAction::SetEnv {
                    key: "DEEPINFRA_API_KEY".into(),
                    value_source: EnvValueSource::FirstOf(vec![
                        EnvValueSource::Literal(String::new()), // check if already set after .env load
                        EnvValueSource::FromCommand(
                            "grep DEEPINFRA_API_KEY .env 2>/dev/null | cut -d= -f2 || echo ''"
                                .into(),
                        ),
                        EnvValueSource::FromCommand(
                            "grep DI_API_KEY .env 2>/dev/null | cut -d= -f2 || echo ''".into(),
                        ),
                    ]),
                },
            ]),
            can_modify_files: false,
        });

        // ── Strategy 2: Permission denied ───────────────────────────────
        registry.add(HealStrategy {
            name: "permission-denied".into(),
            error_pattern: "Permission denied".into(),
            description: "File or directory permission denied — attempt chmod".into(),
            action: HealAction::ProposeCodeChange {
                file: PathBuf::from("(runtime)"),
                description: "Permission denied on file/directory".into(),
                diff_suggestion: "Check file permissions with `ls -la <path>` and run `chmod` or `sudo` as needed.".into(),
            },
            can_modify_files: true,
        });

        // ── Strategy 3: Command not found ───────────────────────────────
        registry.add(HealStrategy {
            name: "command-not-found".into(),
            error_pattern: "command not found".into(),
            description: "Required binary not installed — suggest installation".into(),
            action: HealAction::ProposeCodeChange {
                file: PathBuf::from("(environment)"),
                description: "Required command not found in PATH".into(),
                diff_suggestion: "Install missing command. Common package managers: apt-get, brew, cargo install.".into(),
            },
            can_modify_files: false,
        });

        // ── Strategy 4: File not found (config) ─────────────────────────
        registry.add(HealStrategy {
            name: "config-file-not-found".into(),
            error_pattern: "Failed to read classifier config".into(),
            description: "Classifier config file not found — suggest creating it".into(),
            action: HealAction::ProposeCodeChange {
                file: PathBuf::from("registry/classify/"),
                description: "Classifier config YAML not found in registry".into(),
                diff_suggestion:
                    "Create the missing classifier config in registry/classify/<name>.yaml".into(),
            },
            can_modify_files: true,
        });

        // ── Strategy 5: Network/connection error ────────────────────────
        registry.add(HealStrategy {
            name: "network-error".into(),
            error_pattern: "connection refused".into(),
            description: "Network connection refused — retry with backoff".into(),
            action: HealAction::RetryWithBackoff {
                max_attempts: 3,
                delay_ms: 2000,
            },
            can_modify_files: false,
        });

        // ── Strategy 6: Generic retry (transient failures) ──────────────
        registry.add(HealStrategy {
            name: "transient-retry".into(),
            error_pattern: "timeout|timed out|temporary failure|500|502|503|rate limit".into(),
            description: "Transient failure — retry with backoff".into(),
            action: HealAction::RetryWithBackoff {
                max_attempts: 3,
                delay_ms: 1000,
            },
            can_modify_files: false,
        });

        registry
    }

    /// Add a healing strategy to the registry.
    pub fn add(&mut self, strategy: HealStrategy) {
        self.strategies.push(strategy);
    }

    /// Find a matching strategy for an error message.
    pub fn find_strategy(&self, error: &str) -> Option<&HealStrategy> {
        let error_lower = error.to_lowercase();
        self.strategies.iter().find(|s| {
            let pattern_lower = s.error_pattern.to_lowercase();
            // Check each word in the pattern against the error
            pattern_lower
                .split('|')
                .any(|word| error_lower.contains(word.trim()))
        })
    }
}

// ── Self Healer ─────────────────────────────────────────────────────────────

/// The self-healing engine. Wraps any fallible operation with automatic
/// diagnosis and recovery.
pub struct SelfHealer {
    registry: HealRegistry,
}

impl SelfHealer {
    /// Create a healer with default strategies.
    pub fn new() -> Self {
        Self {
            registry: HealRegistry::with_defaults(),
        }
    }

    /// Create a healer with a custom registry.
    pub fn with_registry(registry: HealRegistry) -> Self {
        Self { registry }
    }

    /// Access the registry for inspection or extension.
    pub fn registry(&self) -> &HealRegistry {
        &self.registry
    }

    /// Mutable access to the registry.
    pub fn registry_mut(&mut self) -> &mut HealRegistry {
        &mut self.registry
    }

    /// Attempt to heal an error. Returns the healing outcome.
    ///
    /// This is the main entry point. Call it when any operation fails,
    /// before deciding whether to retry, degrade, or escalate.
    pub fn attempt(&self, error: &str, context: &HealContext) -> HealOutcome {
        // CNS span: healing attempt started
        tracing::info!(
            target: "cns.heal.attempt",
            operation = %context.operation,
            error = %error,
            "Self-healing attempt started"
        );

        let strategy = match self.registry.find_strategy(error) {
            Some(s) => s,
            None => {
                tracing::warn!(
                    target: "cns.heal.unmatched",
                    operation = %context.operation,
                    error = %error,
                    "No healing strategy matches this error"
                );
                return HealOutcome::Unhealable {
                    reason: format!("No healing strategy matches: {}", error),
                    suggestion: "Review error and add a healing strategy to the registry.".into(),
                    requires_code_change: false,
                };
            }
        };

        tracing::info!(
            target: "cns.heal.strategy",
            strategy = %strategy.name,
            operation = %context.operation,
            "Applying healing strategy"
        );

        match self.execute_action(&strategy.action, context) {
            Ok(result) => {
                if result.success {
                    HealOutcome::Healed {
                        action_taken: strategy.name.clone(),
                        modifications: result.modifications,
                    }
                } else {
                    HealOutcome::Degraded {
                        reason: format!(
                            "Healing strategy '{}' executed but did not resolve the issue: {}",
                            strategy.name, result.output
                        ),
                        fallback_description: strategy.description.clone(),
                    }
                }
            }
            Err(e) => HealOutcome::Unhealable {
                reason: format!("Healing strategy '{}' failed: {}", strategy.name, e),
                suggestion: strategy.description.clone(),
                requires_code_change: matches!(
                    strategy.action,
                    HealAction::ProposeCodeChange { .. }
                ),
            },
        }
    }

    /// Execute a healing action and return the result.
    fn execute_action(
        &self,
        action: &HealAction,
        context: &HealContext,
    ) -> Result<ActionResult, String> {
        match action {
            HealAction::RunCommand {
                command,
                capture_output,
            } => {
                let output = Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .output()
                    .map_err(|e| format!("Failed to run heal command '{}': {}", command, e))?;

                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

                let success = output.status.success();
                let result_text = if *capture_output { stdout } else { stderr };

                Ok(ActionResult {
                    success,
                    output: result_text,
                    modifications: vec![format!("Ran: {}", command)],
                })
            }

            HealAction::SetEnv { key, value_source } => {
                let value = self.resolve_env_value(value_source, context)?;
                if value.is_empty() {
                    return Ok(ActionResult {
                        success: false,
                        output: format!("Could not resolve value for env var '{}'", key),
                        modifications: vec![],
                    });
                }
                unsafe { std::env::set_var(key, &value) };
                Ok(ActionResult {
                    success: true,
                    output: format!(
                        "Set {}={}",
                        key,
                        if value.len() > 20 {
                            &value[..20]
                        } else {
                            &value
                        }
                    ),
                    modifications: vec![format!("Set env var: {}", key)],
                })
            }

            HealAction::LoadDotEnv { search_paths } => {
                let mut loaded = false;
                let mut modifications = Vec::new();

                for path in search_paths {
                    if path.exists() {
                        match dotenvy::from_path(path) {
                            Ok(_) => {
                                loaded = true;
                                modifications.push(format!("Loaded .env from: {}", path.display()));
                                tracing::info!(
                                    target: "cns.heal.dotenv",
                                    path = %path.display(),
                                    "Loaded .env file"
                                );
                            }
                            Err(_) => {
                                // File exists but couldn't be parsed — skip
                            }
                        }
                    }
                }

                Ok(ActionResult {
                    success: loaded,
                    output: if loaded {
                        "Loaded environment from .env file(s)".into()
                    } else {
                        "No .env files found in search paths".into()
                    },
                    modifications,
                })
            }

            HealAction::CreateDefaultFile { path, content } => {
                if path.exists() {
                    return Ok(ActionResult {
                        success: true,
                        output: format!("File already exists: {}", path.display()),
                        modifications: vec![],
                    });
                }
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("mkdir {}: {}", parent.display(), e))?;
                }
                std::fs::write(path, content)
                    .map_err(|e| format!("write {}: {}", path.display(), e))?;
                Ok(ActionResult {
                    success: true,
                    output: format!("Created default file: {}", path.display()),
                    modifications: vec![format!("Created: {}", path.display())],
                })
            }

            HealAction::RetryWithBackoff {
                max_attempts,
                delay_ms,
            } => {
                // Retry is handled by the caller — just signal that retry is possible
                Ok(ActionResult {
                    success: true,
                    output: format!(
                        "Retry strategy: {} attempts, {}ms delay",
                        max_attempts, delay_ms
                    ),
                    modifications: vec![],
                })
            }

            HealAction::ProposeCodeChange {
                file,
                description,
                diff_suggestion,
            } => {
                // Emit CNS event for Curator review
                tracing::warn!(
                    target: "cns.heal.code_change_proposed",
                    file = %file.display(),
                    description = %description,
                    suggestion = %diff_suggestion,
                    "Code change proposed — requires Curator review"
                );
                Ok(ActionResult {
                    success: false, // Code changes require human approval
                    output: format!("Proposed change to {}: {}", file.display(), description),
                    modifications: vec![format!("Proposed code change: {}", description)],
                })
            }

            HealAction::Sequence(actions) => {
                let mut all_modifications = Vec::new();
                let mut last_output = String::new();
                let mut any_success = false;

                for sub_action in actions {
                    match self.execute_action(sub_action, context) {
                        Ok(result) => {
                            all_modifications.extend(result.modifications);
                            last_output = result.output;
                            if result.success {
                                any_success = true;
                            }
                        }
                        Err(e) => {
                            last_output = e;
                        }
                    }
                }

                Ok(ActionResult {
                    success: any_success,
                    output: last_output,
                    modifications: all_modifications,
                })
            }
        }
    }

    /// Resolve an environment variable value from its source.
    #[allow(clippy::only_used_in_recursion)]
    fn resolve_env_value(
        &self,
        source: &EnvValueSource,
        _context: &HealContext,
    ) -> Result<String, String> {
        match source {
            EnvValueSource::Literal(val) => Ok(val.clone()),
            EnvValueSource::FromFile(path) => {
                if !path.exists() {
                    return Ok(String::new());
                }
                std::fs::read_to_string(path)
                    .map(|s| s.lines().next().unwrap_or("").trim().to_string())
                    .map_err(|e| format!("Read {}: {}", path.display(), e))
            }
            EnvValueSource::FromCommand(cmd) => {
                let output = Command::new("sh")
                    .arg("-c")
                    .arg(cmd)
                    .output()
                    .map_err(|e| format!("Command '{}': {}", cmd, e))?;
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
            EnvValueSource::FirstOf(sources) => {
                for source in sources {
                    match self.resolve_env_value(source, _context) {
                        Ok(val) if !val.is_empty() => return Ok(val),
                        _ => continue,
                    }
                }
                Ok(String::new())
            }
        }
    }
}

impl Default for SelfHealer {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for SelfHealer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelfHealer")
            .field("strategies", &self.registry.strategies.len())
            .finish()
    }
}

// ── Convenience wrapper for QA runner integration ───────────────────────────

/// Wrap a fallible operation with self-healing.
///
/// Usage:
/// ```ignore
/// let result = SelfHealer::new().healable(
///     || fallible_operation(),
///     HealContext { operation: "classify", ..Default::default() }
/// );
/// ```
impl SelfHealer {
    /// Execute a fallible operation with automatic healing.
    /// If the operation fails, attempts to heal and retries once.
    /// If healing fails, returns the original error.
    pub fn healable<T, E: fmt::Display>(
        &self,
        operation: impl Fn() -> Result<T, E>,
        context: HealContext,
    ) -> Result<T, E> {
        match operation() {
            Ok(val) => Ok(val),
            Err(e) => {
                let error_str = e.to_string();
                let outcome = self.attempt(&error_str, &context);

                match outcome {
                    HealOutcome::Healed { .. } => {
                        // Retry the operation once after healing
                        operation()
                    }
                    HealOutcome::Degraded { .. } | HealOutcome::Unhealable { .. } => {
                        // Could not heal — return original error
                        Err(e)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_default_strategies() {
        let registry = HealRegistry::with_defaults();
        assert!(
            registry.strategies.len() >= 4,
            "Should have at least 4 default strategies"
        );
    }

    #[test]
    fn find_api_key_strategy() {
        let registry = HealRegistry::with_defaults();
        let strategy = registry.find_strategy("No API key for classifier 'qa-triage'");
        assert!(strategy.is_some(), "Should find API key strategy");
        assert_eq!(strategy.unwrap().name, "missing-api-key");
    }

    #[test]
    fn find_permission_strategy() {
        let registry = HealRegistry::with_defaults();
        let strategy = registry.find_strategy("Permission denied (os error 13)");
        assert!(strategy.is_some());
        assert_eq!(strategy.unwrap().name, "permission-denied");
    }

    #[test]
    fn find_network_strategy() {
        let registry = HealRegistry::with_defaults();
        let strategy = registry.find_strategy("connection refused");
        assert!(strategy.is_some());
        assert_eq!(strategy.unwrap().name, "network-error");
    }

    #[test]
    fn find_transient_strategy() {
        let registry = HealRegistry::with_defaults();
        let strategy = registry.find_strategy("request timed out after 30 seconds");
        assert!(strategy.is_some());
        assert_eq!(strategy.unwrap().name, "transient-retry");
    }

    #[test]
    fn no_match_returns_none() {
        let registry = HealRegistry::with_defaults();
        let strategy = registry.find_strategy("completely unknown error XYZ123");
        assert!(strategy.is_none());
    }

    #[test]
    fn heal_unmatched_error_returns_unhealable() {
        let healer = SelfHealer::new();
        let context = HealContext {
            operation: "test".into(),
            error_message: "unknown error".into(),
            ..Default::default()
        };
        let outcome = healer.attempt("unknown error", &context);
        assert!(matches!(outcome, HealOutcome::Unhealable { .. }));
    }

    #[test]
    fn heal_api_key_attempts_dotenv() {
        let healer = SelfHealer::new();
        let context = HealContext {
            operation: "classify".into(),
            error_message: "No API key for classifier 'qa-triage' — set DEEPINFRA_API_KEY".into(),
            config_search_paths: vec![PathBuf::from(".")],
            ..Default::default()
        };
        let outcome = healer.attempt(
            "No API key for classifier 'qa-triage' — set DEEPINFRA_API_KEY or equivalent",
            &context,
        );
        // The healing may succeed or degrade depending on whether .env exists
        // but it should NOT return Unhealable
        assert!(
            !matches!(outcome, HealOutcome::Unhealable { .. }),
            "API key error should have a healing strategy, got: {:?}",
            outcome
        );
    }
}
