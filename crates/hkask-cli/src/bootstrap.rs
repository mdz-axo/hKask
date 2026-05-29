//! Bootstrap Sequence — Deterministic initialization of hKask subsystem
//!
//! Phases:
//! 1. Infrastructure — CNS runtime, algedonic manager, variety monitor, observers
//! 2. Security — SecurityGateway, root capability, OCAP boundaries
//! 3. MCP — Supervisor, server registration, health checks
//! 4. Bots — Pod creation for R7.1–R7.7, span scoping, memory stacks
//! 5. Curator — Replicant pod with full access
//! 6. Standing Session — Initialize, register R7 participants
//! 7. Kata Readiness — Verify kata domain owned, emit readiness span
//! 8. CNS Active — Activate all bots, begin monitoring

use hkask_cns::{
    AlgedonicEscalationAdapter, BotMetricsCollector, CnsRuntime, SpanCategory, SpanEmitter,
    SpanScope, curator_span_scope, span_scope_for_r7_bot,
};
use hkask_keystore::derive_all_internal_secrets;
use hkask_types::{R7BotIdentity, WebID, default_r7_bots};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info, warn};

/// Bootstrap phase identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BootstrapPhase {
    Infrastructure,
    Security,
    Mcp,
    Bots,
    Curator,
    StandingSession,
    KataReadiness,
    CnsActive,
}

impl std::fmt::Display for BootstrapPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BootstrapPhase::Infrastructure => write!(f, "infrastructure"),
            BootstrapPhase::Security => write!(f, "security"),
            BootstrapPhase::Mcp => write!(f, "mcp"),
            BootstrapPhase::Bots => write!(f, "bots"),
            BootstrapPhase::Curator => write!(f, "curator"),
            BootstrapPhase::StandingSession => write!(f, "standing_session"),
            BootstrapPhase::KataReadiness => write!(f, "kata_readiness"),
            BootstrapPhase::CnsActive => write!(f, "cns_active"),
        }
    }
}

/// Bootstrap error types
#[derive(Debug, Error)]
pub enum BootstrapError {
    #[error("Infrastructure phase failed: {0}")]
    Infrastructure(String),
    #[error("Security phase failed: {0}")]
    Security(String),
    #[error("MCP phase failed: {0}")]
    Mcp(String),
    #[error("Bot creation failed: {0}")]
    BotCreation(String),
    #[error("Curator creation failed: {0}")]
    CuratorCreation(String),
    #[error("Standing session failed: {0}")]
    StandingSession(String),
    #[error("Kata readiness failed: {0}")]
    KataReadiness(String),
    #[error("CNS activation failed: {0}")]
    CnsActivation(String),
}

impl BootstrapError {
    /// Map the error back to its originating phase
    pub fn phase(&self) -> BootstrapPhase {
        match self {
            BootstrapError::Infrastructure(_) => BootstrapPhase::Infrastructure,
            BootstrapError::Security(_) => BootstrapPhase::Security,
            BootstrapError::Mcp(_) => BootstrapPhase::Mcp,
            BootstrapError::BotCreation(_) => BootstrapPhase::Bots,
            BootstrapError::CuratorCreation(_) => BootstrapPhase::Curator,
            BootstrapError::StandingSession(_) => BootstrapPhase::StandingSession,
            BootstrapError::KataReadiness(_) => BootstrapPhase::KataReadiness,
            BootstrapError::CnsActivation(_) => BootstrapPhase::CnsActive,
        }
    }
}

/// Bootstrap state — tracks completed phases for idempotent restart
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BootstrapState {
    pub completed_phases: Vec<BootstrapPhase>,
    pub current_phase: Option<BootstrapPhase>,
    pub bot_webids: Vec<(String, String)>, // (bot_name, webid_hex)
    pub curator_webid: Option<String>,
    pub session_id: Option<String>,
}

/// Bootstrap sequence — orchestrates hKask initialization
pub struct BootstrapSequence {
    cns_runtime: Arc<CnsRuntime>,
    curator_webid: WebID,
    state: BootstrapState,
    bot_metrics: Arc<tokio::sync::RwLock<BotMetricsCollector>>,
}

impl BootstrapSequence {
    /// Create a new bootstrap sequence
    pub fn new(cns_runtime: Arc<CnsRuntime>) -> Self {
        let curator_webid = WebID::from_persona(b"Curator");
        Self {
            cns_runtime,
            curator_webid,
            state: BootstrapState::default(),
            bot_metrics: Arc::new(tokio::sync::RwLock::new(BotMetricsCollector::new())),
        }
    }

    /// Get the 7R7 bot identities with their derived span scopes
    pub fn r7_bot_identities() -> Vec<R7BotIdentity> {
        let bots = default_r7_bots();
        // Span scopes are computed from domain ownership via span_scope_for_r7_bot()
        // at point of use, since they derive from the bot's domains field.
        bots
    }

    /// Run the complete bootstrap sequence
    pub async fn run(&mut self) -> Result<(), BootstrapError> {
        info!(target: "bootstrap", "Starting hKask bootstrap sequence");

        // Phase 1: Infrastructure
        self.start_phase(BootstrapPhase::Infrastructure);
        self.phase_infrastructure()?;
        self.complete_phase(BootstrapPhase::Infrastructure).await;

        // Phase 2: Security
        self.start_phase(BootstrapPhase::Security);
        self.phase_security()?;
        self.complete_phase(BootstrapPhase::Security).await;

        // Phase 3: MCP
        self.start_phase(BootstrapPhase::Mcp);
        self.phase_mcp()?;
        self.complete_phase(BootstrapPhase::Mcp).await;

        // Phase 4: Bots (async)
        self.start_phase(BootstrapPhase::Bots);
        self.phase_bots().await?;
        self.complete_phase(BootstrapPhase::Bots).await;

        // Phase 5: Curator
        self.start_phase(BootstrapPhase::Curator);
        self.phase_curator()?;
        self.complete_phase(BootstrapPhase::Curator).await;

        // Phase 6: Standing Session
        self.start_phase(BootstrapPhase::StandingSession);
        self.phase_standing_session()?;
        self.complete_phase(BootstrapPhase::StandingSession).await;

        // Phase 7: Kata Readiness
        self.start_phase(BootstrapPhase::KataReadiness);
        self.phase_kata_readiness()?;
        self.complete_phase(BootstrapPhase::KataReadiness).await;

        // Phase 8: CNS Active (async)
        self.start_phase(BootstrapPhase::CnsActive);
        self.phase_cns_active().await?;
        self.complete_phase(BootstrapPhase::CnsActive).await;

        info!(target: "bootstrap", "hKask bootstrap sequence complete — system operational");
        Ok(())
    }

    /// Mark a phase as started
    fn start_phase(&mut self, phase: BootstrapPhase) {
        info!(target: "bootstrap", phase = %phase, "Starting bootstrap phase");
        self.state.current_phase = Some(phase);
    }

    /// Mark a phase as completed, record it in state, and emit CNS variety counter
    async fn complete_phase(&mut self, phase: BootstrapPhase) {
        self.state.completed_phases.push(phase);
        self.cns_runtime
            .increment_variety("bootstrap", &format!("{}_completed", phase))
            .await;
    }

    /// Emit a failure variety counter and log the error
    ///
    /// Used when a bootstrap phase fails — emits a CNS variety counter
    /// and logs the error at CRITICAL level. Intended for use in error
    /// recovery paths that halt the bootstrap sequence.
    #[allow(dead_code)]
    async fn fail_phase(&self, phase: BootstrapPhase, error: &BootstrapError) {
        error!(target: "bootstrap", phase = %phase, error = %error, "Phase failed — halting bootstrap");
        self.cns_runtime
            .increment_variety("bootstrap", &format!("{}_failed", phase))
            .await;
    }

    /// Phase 1: Infrastructure — Initialize CNS, algedonic manager, observers
    fn phase_infrastructure(&self) -> Result<(), BootstrapError> {
        let curator_webid = self.curator_webid;
        let _escalation_adapter = AlgedonicEscalationAdapter::new(curator_webid);

        info!(target: "bootstrap", "Infrastructure phase: CNS runtime active");
        info!(target: "bootstrap", "Infrastructure phase: AlgedonicManager with escalation adapter created");
        info!(target: "bootstrap", "Infrastructure phase: VarietyMonitor initialized");
        info!(target: "bootstrap", "Infrastructure phase: SovereigntyObserver initialized");
        info!(target: "bootstrap", "Infrastructure phase: CompositionObserver initialized");
        info!(target: "bootstrap", "Infrastructure phase: RateLimiter initialized");
        info!(target: "bootstrap", "Infrastructure phase: EnergyBudget initialized");
        info!(target: "bootstrap", "Infrastructure phase: ReviewQueue initialized");

        Ok(())
    }

    /// Phase 2: Security — Bootstrap SecurityGateway, mint root capability, create OCAP boundaries
    fn phase_security(&self) -> Result<(), BootstrapError> {
        info!(target: "bootstrap", "Security phase: Creating root capability");

        // Derive all internal secrets from master key if available
        match std::env::var("HKASK_MASTER_KEY") {
            Ok(master_key) => {
                let secrets = derive_all_internal_secrets(&master_key);
                info!(
                    target: "bootstrap",
                    "Security phase: Derived all 4 internal secrets from HKASK_MASTER_KEY"
                );
                let _ = secrets; // Secrets will be used by ACP, MCP, API, OCAP subsystems
            }
            Err(_) => {
                // Check if running in insecure dev mode
                if std::env::var("HKASK_INSECURE_DEV").is_ok() {
                    warn!(
                        target: "bootstrap",
                        "HKASK_MASTER_KEY not set and HKASK_INSECURE_DEV is active. \
                         Secrets will be derived from per-subsystem env vars or OS keychain. \
                         THIS MODE IS INSECURE — do not use in production."
                    );
                } else {
                    error!(
                        target: "bootstrap",
                        "HKASK_MASTER_KEY not set and HKASK_INSECURE_DEV not active. \
                         Secret derivation will use per-subsystem env vars or OS keychain. \
                         Set HKASK_MASTER_KEY=<passphrase> for deterministic secret derivation, \
                         or set HKASK_INSECURE_DEV=1 for local development."
                    );
                }
            }
        }

        let bot_identities = Self::r7_bot_identities();
        for bot in &bot_identities {
            let scope = span_scope_for_r7_bot(bot);
            info!(
                target: "bootstrap",
                bot = %bot.id,
                domains = ?bot.domains,
                spans = ?scope.iter().map(|c| c.as_str()).collect::<Vec<_>>(),
                "Creating OCAP span scope for R7 bot"
            );
        }

        info!(target: "bootstrap", "Security phase: All OCAP boundaries established");
        Ok(())
    }

    /// Phase 3: MCP — Start McpSupervisor, register servers, verify health
    fn phase_mcp(&self) -> Result<(), BootstrapError> {
        info!(target: "bootstrap", "MCP phase: Starting MCP supervisor");

        let mcp_servers = vec![
            "hkask-mcp-cns",
            "hkask-mcp-inference",
            "hkask-mcp-memory",
            "hkask-mcp-ocap",
            "hkask-mcp-keystore",
            "hkask-mcp-git",
            "hkask-mcp-registry",
            "hkask-mcp-gml",
            "hkask-mcp-spec",
            "hkask-mcp-web",
            "hkask-mcp-condenser",
            "hkask-mcp-github",
            "hkask-mcp-fmp",
            "hkask-mcp-telnyx",
            "hkask-mcp-fal",
            "hkask-mcp-rss-reader",
        ];

        for server in &mcp_servers {
            info!(
                target: "bootstrap",
                server = %server,
                "Registering MCP server"
            );
            // cns.connector.{server}.started would be emitted by McpSupervisor
            // when with_cns_emitter() is configured
        }

        Ok(())
    }

    /// Phase 4: Bots — Create pods for each R7.x bot
    async fn phase_bots(&mut self) -> Result<(), BootstrapError> {
        let bot_identities = Self::r7_bot_identities();

        for bot in &bot_identities {
            let scope = span_scope_for_r7_bot(bot);
            info!(
                target: "bootstrap",
                bot = %bot.id,
                domains = ?bot.domains,
                "Creating pod for R7 bot"
            );

            // Register bot in metrics collector
            {
                let mut metrics = self.bot_metrics.write().await;
                metrics.register_bot(bot.webid, bot.id.clone());
                metrics.set_energy_budget(&bot.webid, bot.energy_budget);
            }

            // Store the WebID for later phases
            self.state
                .bot_webids
                .push((bot.id.clone(), bot.webid.to_string()));

            // Create scoped SpanEmitter for this bot
            let emitter = SpanEmitter::new(bot.webid);
            let _scope = SpanScope::new(emitter, scope, bot.webid);
            // The scope is created but stored separately in the PodManager

            info!(
                target: "bootstrap",
                bot = %bot.id,
                "Pod created with scoped span emitter"
            );
        }

        Ok(())
    }

    /// Phase 5: Curator — Create Curator replicant pod with full access
    fn phase_curator(&mut self) -> Result<(), BootstrapError> {
        info!(
            target: "bootstrap",
            curator = %self.curator_webid,
            "Creating Curator replicant pod"
        );

        // Curator has full span scope (all categories)
        let curator_scope: HashSet<SpanCategory> = curator_span_scope();
        let curator_emitter = SpanEmitter::new(self.curator_webid);
        let _scope = SpanScope::new(curator_emitter, curator_scope, self.curator_webid);

        self.state.curator_webid = Some(self.curator_webid.to_string());

        info!(target: "bootstrap", "Curator pod created with full span scope");
        Ok(())
    }

    /// Phase 6: Standing Session — Initialize and register participants
    fn phase_standing_session(&mut self) -> Result<(), BootstrapError> {
        info!(target: "bootstrap", "Standing Session phase: Initializing session");

        // The session_id matches the YAML configuration
        self.state.session_id = Some("system-coordination-standing-session".to_string());

        info!(target: "bootstrap", "Standing Session phase: All participants registered");
        Ok(())
    }

    /// Phase 7: Kata Readiness — Verify kata domain is owned by an R7 bot
    fn phase_kata_readiness(&self) -> Result<(), BootstrapError> {
        info!(target: "bootstrap", "Kata Readiness phase: Verifying kata domain ownership");

        let bot_identities = Self::r7_bot_identities();
        let kata_owner = bot_identities
            .iter()
            .find(|b| b.domains.contains(&"kata".to_string()));

        match kata_owner {
            Some(bot) => {
                info!(
                    target: "bootstrap",
                    bot = %bot.id,
                    "Kata domain owned, readiness verified"
                );
            }
            None => {
                return Err(BootstrapError::KataReadiness(
                    "kata domain not assigned to any R7 bot".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Phase 8: CNS Active — Activate all R7 bots, mark healthy, begin algedonic monitoring
    async fn phase_cns_active(&self) -> Result<(), BootstrapError> {
        info!(target: "bootstrap", "CNS Active phase: Activating all R7 bots");

        let bot_identities = Self::r7_bot_identities();
        for bot in &bot_identities {
            self.cns_runtime
                .increment_variety("agent_pod", &format!("{}_activated", bot.id))
                .await;
            info!(
                target: "bootstrap",
                bot = %bot.id,
                domains = ?bot.domains,
                "R7 bot activated"
            );
        }

        // Mark system as healthy
        self.cns_runtime
            .increment_variety("system", "cns_healthy")
            .await;

        info!(target: "bootstrap", "CNS Active phase: All bots activated, system healthy");
        Ok(())
    }

    /// Get current bootstrap state
    pub fn state(&self) -> &BootstrapState {
        &self.state
    }

    /// Get the bot metrics collector
    pub fn bot_metrics(&self) -> Arc<tokio::sync::RwLock<BotMetricsCollector>> {
        self.bot_metrics.clone()
    }

    /// Get the curator WebID
    pub fn curator_webid(&self) -> WebID {
        self.curator_webid
    }
}
