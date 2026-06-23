//! Self-Healing Engine — two-stage autonomous error recovery, integrated into
//! the error-handling foundation layer.
//!
//! Every fallible operation in hKask can pass through a `SelfHealer`. The healer
//! maps error patterns to recovery strategies, executes healing actions, and
//! returns Healed (retry), Degraded (fallback), or Unhealable (escalate to Curator).
//!
//! **Stage 1 (always available):** Deterministic env/config healing — `RunCommand`,
//! `SetEnv`, `LoadDotEnv`, `CreateDefaultFile`, `RetryWithBackoff`, `ProposeCodeChange`.
//! No inference required.
//!
//! **Stage 2 (requires `with_inference()`):** LLM template-assisted healing via
//! `LlmAssisted`. Renders a Jinja2 template from `registry/templates/heal/`,
//! calls the classifier model, parses JSON instructions, executes sub-actions.
//!
//! ## CNS spans emitted
//!
//! | Target | When |
//! |--------|------|
//! | `cns.heal.attempt` | Healing starts |
//! | `cns.heal.strategy` | Strategy selected |
//! | `cns.heal.dotenv` | .env loaded |
//! | `cns.heal.set_env` | Env var set |
//! | `cns.heal.file_created` | File created |
//! | `cns.heal.code_change_proposed` | Code change proposed |
//! | `cns.heal.llm_assisted` | LLM template rendered |
//! | `cns.heal.retry_loop` | Each retry iteration |
//! | `cns.heal.unmatched` | No strategy found |
//! | `cns.heal.escalated` | Exhausted, escalating to Curator |

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

// ── Core types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum HealOutcome {
    Healed {
        action_taken: String,
        modifications: Vec<String>,
    },
    Degraded {
        reason: String,
        fallback_description: String,
    },
    Unhealable {
        reason: String,
        suggestion: String,
        requires_code_change: bool,
        debug_log: MiniDebugLog,
    },
}

#[derive(Debug, Clone, Default)]
pub struct HealContext {
    pub operation: String,
    pub error_message: String,
    pub env_vars: HashMap<String, String>,
    pub config_search_paths: Vec<PathBuf>,
    pub can_retry: bool,
}

#[derive(Debug, Clone)]
pub struct HealStrategy {
    pub name: String,
    pub error_pattern: String,
    pub description: String,
    pub action: HealAction,
    pub can_modify_files: bool,
}

#[derive(Debug, Clone)]
pub enum HealAction {
    RunCommand {
        command: String,
        capture_output: bool,
    },
    SetEnv {
        key: String,
        value_source: EnvValueSource,
    },
    LoadDotEnv {
        search_paths: Vec<PathBuf>,
    },
    CreateDefaultFile {
        path: PathBuf,
        content: String,
    },
    RetryWithBackoff {
        max_attempts: u32,
        delay_ms: u64,
    },
    ProposeCodeChange {
        file: PathBuf,
        description: String,
        diff_suggestion: String,
    },
    Sequence(Vec<HealAction>),
    LlmAssisted {
        template_path: PathBuf,
    },
}

#[derive(Debug, Clone)]
pub enum EnvValueSource {
    Literal(String),
    FromFile(PathBuf),
    FromCommand(String),
    FirstOf(Vec<EnvValueSource>),
}

pub type HealInferenceFn = Box<dyn Fn(&str) -> Result<String, String> + Send + Sync>;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MiniDebugLog {
    pub attempt_count: u32,
    pub cns_spans: Vec<String>,
    pub modifications: Vec<String>,
    pub actions_taken: Vec<DebugLogAction>,
    pub suggestion: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DebugLogAction {
    pub name: String,
    pub output: String,
    pub success: bool,
}

#[derive(Debug, Clone)]
struct ActionResult {
    success: bool,
    output: String,
    modifications: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct HealInstruction {
    action: String,
    #[serde(default)]
    command: String,
    #[serde(default)]
    key: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    content: String,
}

// ── Heal Registry ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct HealRegistry {
    strategies: Vec<HealStrategy>,
}

impl HealRegistry {
    pub fn with_defaults() -> Self {
        let mut r = Self::default();
        r.add(HealStrategy {
            name: "missing-api-key".into(),
            error_pattern: "No API key".into(),
            description: "API key not found — try loading from .env files".into(),
            action: HealAction::LoadDotEnv {
                search_paths: vec![
                    PathBuf::from(".env"),
                    PathBuf::from("../.env"),
                    PathBuf::from("../../.env"),
                    dirs::home_dir()
                        .unwrap_or_default()
                        .join(".config/hkask/.env"),
                ],
            },
            can_modify_files: false,
        });
        r.add(HealStrategy {
            name: "permission-denied".into(),
            error_pattern: "Permission denied".into(),
            description: "Permission denied — check filesystem".into(),
            action: HealAction::ProposeCodeChange {
                file: PathBuf::from("(runtime)"),
                description: "Permission denied".into(),
                diff_suggestion: "Check with `ls -la` and `chmod` or `sudo` as needed.".into(),
            },
            can_modify_files: true,
        });
        r.add(HealStrategy {
            name: "command-not-found".into(),
            error_pattern: "command not found".into(),
            description: "Required binary not installed".into(),
            action: HealAction::ProposeCodeChange {
                file: PathBuf::from("(environment)"),
                description: "Command not found in PATH".into(),
                diff_suggestion: "Install via apt-get, brew, or cargo install.".into(),
            },
            can_modify_files: false,
        });
        r.add(HealStrategy {
            name: "config-file-not-found".into(),
            error_pattern: "Failed to read classifier config".into(),
            description: "Classifier config missing".into(),
            action: HealAction::ProposeCodeChange {
                file: PathBuf::from("registry/classify/"),
                description: "Classifier config YAML not found".into(),
                diff_suggestion: "Create registry/classify/<name>.yaml".into(),
            },
            can_modify_files: true,
        });
        r.add(HealStrategy {
            name: "network-error".into(),
            error_pattern: "connection refused".into(),
            description: "Network error — retry with backoff".into(),
            action: HealAction::RetryWithBackoff {
                max_attempts: 3,
                delay_ms: 2000,
            },
            can_modify_files: false,
        });
        r.add(HealStrategy {
            name: "transient-retry".into(),
            error_pattern: "timeout|timed out|temporary failure|500|502|503|rate limit".into(),
            description: "Transient failure — retry with backoff".into(),
            action: HealAction::RetryWithBackoff {
                max_attempts: 3,
                delay_ms: 1000,
            },
            can_modify_files: false,
        });
        r
    }

    pub fn add(&mut self, s: HealStrategy) {
        self.strategies.push(s);
    }

    pub fn find_strategy(&self, error: &str) -> Option<&HealStrategy> {
        let lower = error.to_lowercase();
        self.strategies.iter().find(|s| {
            s.error_pattern
                .to_lowercase()
                .split('|')
                .any(|w| lower.contains(w.trim()))
        })
    }
}

// ── SelfHealer ─────────────────────────────────────────────────────────────

const MAX_RETRIES: u32 = 3;

pub struct SelfHealer {
    registry: HealRegistry,
    inference: Option<HealInferenceFn>,
}

impl SelfHealer {
    pub fn new() -> Self {
        Self {
            registry: HealRegistry::with_defaults(),
            inference: None,
        }
    }

    pub fn with_registry(registry: HealRegistry) -> Self {
        Self {
            registry,
            inference: None,
        }
    }

    pub fn with_inference(mut self, f: HealInferenceFn) -> Self {
        self.inference = Some(f);
        self
    }

    pub fn registry(&self) -> &HealRegistry {
        &self.registry
    }
    pub fn registry_mut(&mut self) -> &mut HealRegistry {
        &mut self.registry
    }
    pub fn has_inference(&self) -> bool {
        self.inference.is_some()
    }

    /// Wrap a fallible operation with automatic healing and bounded retries.
    ///
    /// 3 attempts with exponential backoff (1s → 2s → 4s).
    /// Emits `cns.heal.retry_loop` per iteration, `cns.heal.escalated` on exhaustion.
    pub fn healable<T, E: fmt::Display>(
        &self,
        mut operation: impl FnMut() -> Result<T, E>,
        context: HealContext,
    ) -> Result<T, E> {
        let base_delay_ms: u64 = 1000;
        let mut debug_log = MiniDebugLog::default();

        for attempt in 1..=MAX_RETRIES {
            match operation() {
                Ok(v) => return Ok(v),
                Err(e) => {
                    let err = e.to_string();
                    tracing::info!(
                        target: "cns.heal.retry_loop",
                        attempt,
                        max_retries = MAX_RETRIES,
                        operation = %context.operation,
                        error = %err,
                    );

                    match self.attempt(&err, &context) {
                        HealOutcome::Healed {
                            action_taken,
                            modifications,
                        } => {
                            debug_log.modifications.extend(modifications);
                            debug_log.actions_taken.push(DebugLogAction {
                                name: action_taken,
                                output: "Healed — retrying".into(),
                                success: true,
                            });
                            thread::sleep(Duration::from_millis(
                                base_delay_ms * (1u64 << (attempt - 1)),
                            ));
                            continue;
                        }
                        HealOutcome::Degraded { reason, .. } => {
                            debug_log.attempt_count = attempt;
                            if attempt < MAX_RETRIES {
                                thread::sleep(Duration::from_millis(
                                    base_delay_ms * (1u64 << (attempt - 1)),
                                ));
                                continue;
                            }
                            debug_log.modifications.push(reason);
                            break;
                        }
                        HealOutcome::Unhealable {
                            reason,
                            suggestion,
                            debug_log: attempt_log,
                            ..
                        } => {
                            debug_log.attempt_count = attempt;
                            debug_log.cns_spans.extend(attempt_log.cns_spans);
                            debug_log.modifications.extend(attempt_log.modifications);
                            debug_log.actions_taken.extend(attempt_log.actions_taken);
                            debug_log.suggestion = suggestion.clone();
                            if attempt < MAX_RETRIES {
                                thread::sleep(Duration::from_millis(
                                    base_delay_ms * (1u64 << (attempt - 1)),
                                ));
                                continue;
                            }
                            let json = serde_json::to_string(&debug_log).unwrap_or_default();
                            tracing::error!(
                                target: "cns.heal.escalated",
                                operation = %context.operation,
                                reason = %reason,
                                debug_log = %json,
                                "Healing exhausted — escalating to Curator"
                            );
                            return Err(e);
                        }
                    }
                }
            }
        }

        match operation() {
            Ok(v) => {
                tracing::info!(
                    target: "cns.heal.retry_loop",
                    operation = %context.operation,
                    "Operation succeeded after healing exhaustion"
                );
                Ok(v)
            }
            Err(e) => {
                let json = serde_json::to_string(&debug_log).unwrap_or_default();
                tracing::error!(
                    target: "cns.heal.escalated",
                    operation = %context.operation,
                    attempt_count = debug_log.attempt_count,
                    debug_log = %json,
                    "Healing exhausted — escalating to Curator"
                );
                Err(e)
            }
        }
    }

    /// Attempt to heal an error. Returns Healed, Degraded, or Unhealable.
    pub fn attempt(&self, error: &str, context: &HealContext) -> HealOutcome {
        tracing::info!(
            target: "cns.heal.attempt",
            operation = %context.operation,
            error = %error,
            cns_span = %hkask_types::cns::CnsSpan::SelfHeal,
        );

        let strategy = match self.registry.find_strategy(error) {
            Some(s) => s,
            None => {
                tracing::warn!(
                    target: "cns.heal.unmatched",
                    operation = %context.operation,
                    error = %error,
                );
                return HealOutcome::Unhealable {
                    reason: format!("No healing strategy matches: {}", error),
                    suggestion:
                        "Add a healing strategy to HealRegistry or a template to registry/templates/heal/"
                            .into(),
                    requires_code_change: false,
                    debug_log: MiniDebugLog {
                        attempt_count: 1,
                        cns_spans: vec!["cns.heal.unmatched".into()],
                        suggestion: "Add strategy or template".into(),
                        ..Default::default()
                    },
                };
            }
        };

        tracing::info!(
            target: "cns.heal.strategy",
            strategy = %strategy.name,
            operation = %context.operation,
        );

        match self.execute_action(&strategy.action, context) {
            Ok(result) if result.success => HealOutcome::Healed {
                action_taken: strategy.name.clone(),
                modifications: result.modifications,
            },
            Ok(result) => HealOutcome::Degraded {
                reason: format!(
                    "Strategy '{}' did not resolve the issue: {}",
                    strategy.name, result.output
                ),
                fallback_description: strategy.description.clone(),
            },
            Err(e) => HealOutcome::Unhealable {
                reason: format!("Strategy '{}' failed: {}", strategy.name, e),
                suggestion: strategy.description.clone(),
                requires_code_change: matches!(
                    strategy.action,
                    HealAction::ProposeCodeChange { .. }
                ),
                debug_log: MiniDebugLog {
                    attempt_count: 1,
                    cns_spans: vec!["cns.heal.strategy".into()],
                    actions_taken: vec![DebugLogAction {
                        name: strategy.name.clone(),
                        output: e.clone(),
                        success: false,
                    }],
                    suggestion: strategy.description.clone(),
                    ..Default::default()
                },
            },
        }
    }

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
                let out = Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .output()
                    .map_err(|e| format!("{}: {}", command, e))?;
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                Ok(ActionResult {
                    success: out.status.success(),
                    output: if *capture_output { stdout } else { stderr },
                    modifications: vec![format!("Ran: {}", command)],
                })
            }

            HealAction::SetEnv { key, value_source } => {
                let value = resolve_env_value(value_source)?;
                if value.is_empty() {
                    return Ok(ActionResult {
                        success: false,
                        output: format!("Could not resolve '{}'", key),
                        modifications: vec![],
                    });
                }
                unsafe { std::env::set_var(key, &value) };
                tracing::info!(target: "cns.heal.set_env", key = %key);
                Ok(ActionResult {
                    success: true,
                    output: format!("Set {}", key),
                    modifications: vec![format!("Set env: {}", key)],
                })
            }

            HealAction::LoadDotEnv { search_paths } => {
                let mut loaded = false;
                let mut mods = Vec::new();
                for path in search_paths {
                    if path.exists() {
                        if dotenvy::from_path(path).is_ok() {
                            loaded = true;
                            mods.push(format!("Loaded .env from: {}", path.display()));
                            tracing::info!(
                                target: "cns.heal.dotenv",
                                path = %path.display(),
                            );
                        }
                    }
                }
                Ok(ActionResult {
                    success: loaded,
                    output: if loaded {
                        "Loaded .env".into()
                    } else {
                        "No .env found".into()
                    },
                    modifications: mods,
                })
            }

            HealAction::CreateDefaultFile { path, content } => {
                if path.exists() {
                    return Ok(ActionResult {
                        success: true,
                        output: format!("Already exists: {}", path.display()),
                        modifications: vec![],
                    });
                }
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("mkdir {}: {}", parent.display(), e))?;
                }
                std::fs::write(path, content)
                    .map_err(|e| format!("write {}: {}", path.display(), e))?;
                tracing::info!(
                    target: "cns.heal.file_created",
                    path = %path.display(),
                );
                Ok(ActionResult {
                    success: true,
                    output: format!("Created: {}", path.display()),
                    modifications: vec![format!("Created: {}", path.display())],
                })
            }

            HealAction::RetryWithBackoff {
                max_attempts,
                delay_ms,
            } => Ok(ActionResult {
                success: true,
                output: format!("Retry: {} attempts, {}ms base", max_attempts, delay_ms),
                modifications: vec![],
            }),

            HealAction::ProposeCodeChange {
                file,
                description,
                diff_suggestion,
            } => {
                tracing::warn!(
                    target: "cns.heal.code_change_proposed",
                    file = %file.display(),
                    description = %description,
                    suggestion = %diff_suggestion,
                );
                Ok(ActionResult {
                    success: false,
                    output: format!("Proposed: {} — {}", file.display(), description),
                    modifications: vec![format!("Proposed: {}", description)],
                })
            }

            HealAction::Sequence(actions) => {
                let mut mods = Vec::new();
                let mut last = String::new();
                let mut any = false;
                for a in actions {
                    match self.execute_action(a, context) {
                        Ok(r) => {
                            mods.extend(r.modifications);
                            last = r.output;
                            if r.success {
                                any = true;
                            }
                        }
                        Err(e) => last = e,
                    }
                }
                Ok(ActionResult {
                    success: any,
                    output: last,
                    modifications: mods,
                })
            }

            HealAction::LlmAssisted { template_path } => {
                let inference = self.inference.as_ref().ok_or_else(|| {
                    "No inference wired — use SelfHealer::with_inference()".to_string()
                })?;

                let prompt = render_heal_template(template_path, context)?;

                tracing::info!(
                    target: "cns.heal.llm_assisted",
                    template = %template_path.display(),
                    operation = %context.operation,
                );

                let response =
                    inference(&prompt).map_err(|e| format!("Inference call failed: {}", e))?;
                let instructions: Vec<HealInstruction> = parse_llm_response(&response)?;

                if instructions.is_empty() {
                    return Ok(ActionResult {
                        success: false,
                        output: "LLM returned no instructions".into(),
                        modifications: vec![],
                    });
                }

                let mut mods = Vec::new();
                let mut last = String::new();
                let mut any = false;

                for instr in &instructions {
                    match instr.action.as_str() {
                        "RunCommand" if !instr.command.is_empty() => {
                            let out = Command::new("sh")
                                .arg("-c")
                                .arg(&instr.command)
                                .output()
                                .map_err(|e| format!("{}: {}", instr.command, e))?;
                            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                            let ok = out.status.success();
                            mods.push(format!(
                                "{}: {}",
                                if ok { "Ran" } else { "Failed" },
                                instr.command
                            ));
                            last = stdout;
                            if ok {
                                any = true;
                            }
                        }
                        "SetEnv" if !instr.key.is_empty() => {
                            unsafe { std::env::set_var(&instr.key, &instr.value) };
                            mods.push(format!("Set env: {}", instr.key));
                            tracing::info!(
                                target: "cns.heal.set_env",
                                key = %instr.key,
                            );
                            any = true;
                            last = format!("Set {}", instr.key);
                        }
                        "CreateDefaultFile" if !instr.path.is_empty() => {
                            let p = std::path::Path::new(&instr.path);
                            if !p.exists() {
                                if let Some(parent) = p.parent() {
                                    let _ = std::fs::create_dir_all(parent);
                                }
                                let _ = std::fs::write(p, &instr.content);
                                mods.push(format!("Created: {}", instr.path));
                                any = true;
                                last = format!("Created: {}", instr.path);
                            }
                        }
                        _ => last = format!("Unknown: {}", instr.action),
                    }
                }

                Ok(ActionResult {
                    success: any,
                    output: last,
                    modifications: mods,
                })
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
            .field("has_inference", &self.inference.is_some())
            .finish()
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn resolve_env_value(source: &EnvValueSource) -> Result<String, String> {
    match source {
        EnvValueSource::Literal(v) => Ok(v.clone()),
        EnvValueSource::FromFile(p) => {
            if !p.exists() {
                return Ok(String::new());
            }
            std::fs::read_to_string(p)
                .map(|s| s.lines().next().unwrap_or("").trim().to_string())
                .map_err(|e| format!("Read {}: {}", p.display(), e))
        }
        EnvValueSource::FromCommand(cmd) => {
            let out = Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .output()
                .map_err(|e| format!("{}: {}", cmd, e))?;
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        }
        EnvValueSource::FirstOf(sources) => {
            for s in sources {
                if let Ok(v) = resolve_env_value(s) {
                    if !v.is_empty() {
                        return Ok(v);
                    }
                }
            }
            Ok(String::new())
        }
    }
}

fn render_heal_template(path: &PathBuf, ctx: &HealContext) -> Result<String, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Read {}: {}", path.display(), e))?;

    let mut vars: HashMap<String, String> = HashMap::new();
    vars.insert("operation".into(), ctx.operation.clone());
    vars.insert("error".into(), ctx.error_message.clone());
    vars.insert("error_message".into(), ctx.error_message.clone());
    vars.insert(
        "config_search_paths".into(),
        ctx.config_search_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(":"),
    );
    if !ctx.env_vars.is_empty() {
        vars.insert(
            "env_vars".into(),
            ctx.env_vars
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }

    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Lenient);
    env.add_template("heal", &content)
        .map_err(|e| format!("Invalid template: {}", e))?;

    let cv = serde_json::to_value(&vars).map_err(|e| format!("Serialize: {}", e))?;
    let jc = minijinja::Value::from_serialize(&cv);

    env.get_template("heal")
        .and_then(|t| t.render(jc))
        .map_err(|e| format!("Render: {}", e))
}

fn parse_llm_response(raw: &str) -> Result<Vec<HealInstruction>, String> {
    let t = raw.trim();

    if let Ok(v) = serde_json::from_str::<Vec<HealInstruction>>(t) {
        return Ok(v);
    }
    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(t) {
        if let Some(arr) = obj.get("actions").and_then(|v| v.as_array()) {
            return arr
                .iter()
                .map(|v| serde_json::from_value(v.clone()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("{}", e));
        }
    }
    for fence in &["```json", "```"] {
        if let Some(start) = t.find(fence) {
            let after = &t[start + fence.len()..];
            if let Some(end) = after.find("```") {
                if let Ok(v) = serde_json::from_str::<Vec<HealInstruction>>(&after[..end]) {
                    return Ok(v);
                }
            }
        }
    }
    Err(format!("Not valid JSON. Got: {}", &t[..t.len().min(200)]))
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_default_strategies() {
        assert!(HealRegistry::with_defaults().strategies.len() >= 4);
    }

    #[test]
    fn find_api_key_strategy() {
        let r = HealRegistry::with_defaults();
        let s = r.find_strategy("No API key for classifier 'qa-triage'");
        assert_eq!(s.unwrap().name, "missing-api-key");
    }

    #[test]
    fn find_permission_strategy() {
        let r = HealRegistry::with_defaults();
        assert_eq!(
            r.find_strategy("Permission denied (os error 13)")
                .unwrap()
                .name,
            "permission-denied"
        );
    }

    #[test]
    fn find_network_strategy() {
        let r = HealRegistry::with_defaults();
        assert_eq!(
            r.find_strategy("connection refused").unwrap().name,
            "network-error"
        );
    }

    #[test]
    fn find_transient_strategy() {
        let r = HealRegistry::with_defaults();
        assert!(
            r.find_strategy("request timed out after 30 seconds")
                .is_some()
        );
        assert!(r.find_strategy("HTTP 502 Bad Gateway").is_some());
    }

    #[test]
    fn no_match_returns_none() {
        assert!(
            HealRegistry::with_defaults()
                .find_strategy("unknown XYZ")
                .is_none()
        );
    }

    #[test]
    fn unmatched_returns_unhealable_with_debug_log() {
        let h = SelfHealer::new();
        let o = h.attempt("unknown error", &HealContext::default());
        assert!(matches!(o, HealOutcome::Unhealable { .. }));
        if let HealOutcome::Unhealable { debug_log, .. } = o {
            assert!(
                debug_log
                    .cns_spans
                    .contains(&"cns.heal.unmatched".to_string())
            );
        }
    }

    #[test]
    fn api_key_strategy_loads_dotenv() {
        let h = SelfHealer::new();
        let o = h.attempt("No API key for classifier", &HealContext::default());
        assert!(
            !matches!(o, HealOutcome::Unhealable { .. }),
            "API key should have a strategy"
        );
    }

    #[test]
    fn healable_retries_with_backoff() {
        use std::time::Instant;
        let h = SelfHealer::new();
        let mut calls = 0u32;
        let start = Instant::now();
        let r: Result<u32, &str> = h.healable(
            || {
                calls += 1;
                if calls < 3 { Err("timeout") } else { Ok(42) }
            },
            HealContext {
                operation: "test".into(),
                error_message: "timeout".into(),
                ..Default::default()
            },
        );
        assert!(r.is_ok());
        assert_eq!(calls, 3);
        assert!(
            start.elapsed().as_millis() >= 2900,
            "Expected >= 3s backoff"
        );
    }

    #[test]
    fn healable_exhausted_returns_error() {
        let h = SelfHealer::new();
        assert!(
            h.healable(
                || Err::<u32, _>("connection refused"),
                HealContext::default()
            )
            .is_err()
        );
    }

    #[test]
    fn mini_debug_log_serializes() {
        let log = MiniDebugLog {
            attempt_count: 3,
            cns_spans: vec!["cns.heal.attempt".into()],
            modifications: vec!["Loaded .env".into()],
            actions_taken: vec![DebugLogAction {
                name: "x".into(),
                output: "ok".into(),
                success: true,
            }],
            suggestion: "fix".into(),
        };
        let json = serde_json::to_string(&log).unwrap();
        assert!(json.contains("attempt_count"));
    }

    #[test]
    fn llm_assisted_without_inference_returns_error() {
        let h = SelfHealer::new();
        let action = HealAction::LlmAssisted {
            template_path: PathBuf::from("nonexistent.j2"),
        };
        let r = h.execute_action(&action, &HealContext::default());
        assert!(r.is_err());
    }

    #[test]
    fn heal_templates_exist_on_disk() {
        // CARGO_MANIFEST_DIR = crates/hkask-services-core
        // Go up 2 levels to reach project root
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        for tpl in &[
            "missing_api_key.j2",
            "permission_denied.j2",
            "command_not_found.j2",
            "config_not_found.j2",
            "network_error.j2",
            "transient_retry.j2",
        ] {
            assert!(
                root.join("registry/templates/heal").join(tpl).exists(),
                "Missing: {}",
                tpl
            );
        }
    }
}
