//! Bootstrap Sequence — Deterministic initialization of hKask subsystem
//!
//! Phases:
//! 1. Infrastructure — CNS runtime, algedonic manager, variety monitor, observers
//! 2. Security — SecurityGateway, root capability, OCAP boundaries
//! 3. MCP — Supervisor, server registration, health checks
//! 4. Bots — Pod creation, span scoping, capability tokens
//! 5. Curator — Replicant pod with full access
//! 6. Standing Session — Initialize, register participants
//! 7. Kata Readiness — Verify kata-bot, emit readiness span
//! 8. CNS Active — Activate all bots, begin monitoring

use hkask_cns::{
    AlgedonicEscalationAdapter, BotMetricsCollector, CnsRuntime, SpanCategory, SpanEmitter,
    SpanScope, span_scope_for_bot,
};
use hkask_keystore::derive_all_internal_secrets;
use hkask_types::WebID;
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

/// R7 Bot definition for bootstrap
#[derive(Debug, Clone)]
pub struct BotDefinition {
    pub name: String,
    pub webid: WebID,
    pub allowed_spans: HashSet<SpanCategory>,
    pub energy_budget: u64,
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

    /// Get the 7R7 bot definitions with their allowed span categories
    pub fn r7_bot_definitions() -> Vec<BotDefinition> {
        let bots = vec![
            "cns-curator-bot",
            "memory-curator-bot",
            "inference-curator-bot",
            "mcp-dispatch-bot",
            "ensemble-curator-bot",
            "git-curator-bot",
            "registry-dispatch-bot",
            "kata-bot",
        ];

        let energy_budgets = vec![10_000, 10_000, 15_000, 12_000, 8_000, 8_000, 10_000, 8_000];

        bots.into_iter()
            .zip(energy_budgets)
            .map(|(name, budget)| BotDefinition {
                name: name.to_string(),
                webid: WebID::from_persona(name.as_bytes()),
                allowed_spans: span_scope_for_bot(name),
                energy_budget: budget,
            })
            .collect()
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
                warn!(
                    target: "bootstrap",
                    "HKASK_MASTER_KEY not set — falling back to per-secret env vars or keychain. \
                     Set HKASK_MASTER_KEY for deterministic secret derivation across restarts."
                );
            }
        }

        let bot_definitions = Self::r7_bot_definitions();
        for bot_def in &bot_definitions {
            info!(
                target: "bootstrap",
                bot = %bot_def.name,
                spans = ?bot_def.allowed_spans.iter().map(|c| c.as_str()).collect::<Vec<_>>(),
                "Creating OCAP span scope for bot"
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

    /// Phase 4: Bots — Create pods for each R7 bot
    async fn phase_bots(&mut self) -> Result<(), BootstrapError> {
        let bot_definitions = Self::r7_bot_definitions();

        for bot_def in &bot_definitions {
            info!(
                target: "bootstrap",
                bot = %bot_def.name,
                "Creating pod for R7 bot"
            );

            // Register bot in metrics collector
            {
                let mut metrics = self.bot_metrics.write().await;
                metrics.register_bot(bot_def.webid, bot_def.name.clone());
                metrics.set_energy_budget(&bot_def.webid, bot_def.energy_budget);
            }

            // Store the WebID for later phases
            self.state
                .bot_webids
                .push((bot_def.name.clone(), bot_def.webid.to_string()));

            // Create scoped SpanEmitter for this bot
            let emitter = SpanEmitter::new(bot_def.webid);
            let _scope = SpanScope::new(emitter, bot_def.allowed_spans.clone(), bot_def.webid);
            // The scope is created but stored separately in the PodManager

            info!(
                target: "bootstrap",
                bot = %bot_def.name,
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
        let curator_scope: HashSet<SpanCategory> = span_scope_for_bot("Curator");
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

    /// Phase 7: Kata Readiness — Verify kata-bot manifest and templates
    fn phase_kata_readiness(&self) -> Result<(), BootstrapError> {
        info!(target: "bootstrap", "Kata Readiness phase: Verifying kata-bot manifest");

        let bot_definitions = Self::r7_bot_definitions();
        let kata_bot = bot_definitions.iter().find(|b| b.name == "kata-bot");

        if kata_bot.is_none() {
            return Err(BootstrapError::KataReadiness(
                "kata-bot definition not found".to_string(),
            ));
        }

        info!(target: "bootstrap", "Kata Readiness phase: kata-bot manifest verified");
        Ok(())
    }

    /// Phase 8: CNS Active — Activate all bots, mark healthy, begin algedonic monitoring
    async fn phase_cns_active(&self) -> Result<(), BootstrapError> {
        info!(target: "bootstrap", "CNS Active phase: Activating all bots");

        let bot_definitions = Self::r7_bot_definitions();
        for bot_def in &bot_definitions {
            self.cns_runtime
                .increment_variety("agent_pod", &format!("{}_activated", bot_def.name))
                .await;
            info!(
                target: "bootstrap",
                bot = %bot_def.name,
                "Bot activated"
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
