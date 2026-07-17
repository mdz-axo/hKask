//! Dependency injection traits for the turn loop.
//!
//! Defines the 4 capability traits + 3 closures that `run_turn_loop` needs,
//! plus production adapters that bridge these traits to the existing
//! `ReplState` / `AgentService` / `EnergyGuard` infrastructure.
//!
//! The trait abstractions exist to make the turn loop testable. Each trait
//! has two implementors: a production adapter (here) and a mock (in
//! `turn.rs` test module). Bugs in the loop's behavioral logic — the
//! `if iteration == 1` display bug, the gas leak on inference error, the
//! `mark_seeded` regression — are all catchable by mock-based tests.

use std::sync::Arc;

use hkask_cns::GovernedTool;
use hkask_mcp::RawMcpToolPort;
use hkask_ports::StructuredToolCall;
use hkask_services_chat::{ChatService, TurnRequest, TurnResult};
use hkask_services_context::AgentService;
use hkask_services_core::ServiceError;
use hkask_types::WebID;

use super::energy::EnergyGuard;
use super::threads::ThreadRegistry;
use super::tool_augmented::ToolCall;

// ── Capability traits ────────────────────────────────────────────────

/// Execute inference turns with memory recall, persona filtering, and
/// manifest cascade. Abstracts `ChatService::execute_turn`.
///
/// Production: wraps `ChatService::execute_turn(&service_context, ...)`.
/// Mock: returns predetermined `TurnResult`s in sequence.
#[async_trait::async_trait]
pub trait TurnExecutor: Send + Sync {
    async fn execute_turn(&self, req: &TurnRequest) -> Result<TurnResult, ServiceError>;
}

/// Reserve and settle gas for inference. Abstracts `EnergyGuard` +
/// `CyberneticsLoop` gas accounting.
///
/// Production: wraps `EnergyGuard::try_reserve` + `gas_remaining`/`gas_cap`.
/// Mock: tracks reservations and settlements in memory.
pub trait GasGovernor: Send + Sync {
    /// Reserve gas. Returns `None` if budget exhausted.
    fn try_reserve(&self, heuristic: u64) -> Option<Box<dyn GasReservation>>;
    /// Current gas status: (remaining, cap).
    fn gas_status(&self) -> (u64, u64);
}

/// A gas reservation that must be settled or released.
/// Wraps `EnergyGuard`'s owned-consumption pattern behind `&mut self`
/// for trait-object safety. The underlying `EnergyGuard::Drop` still
/// logs if neither `settle` nor `release` is called.
pub trait GasReservation: Send {
    fn heuristic(&self) -> u64;
    fn settle(&mut self, actual: u64);
    fn release(&mut self);
}

/// Invoke tool calls through governance (OCAP + energy + CNS).
/// Abstracts `invoke_tool_call` with token minting.
///
/// Production: wraps `GovernedTool::invoke` with `DelegationToken`.
/// Mock: returns predetermined `Value`s per tool name.
#[async_trait::async_trait]
pub trait ToolInvoker: Send + Sync {
    async fn invoke(&self, call: &ToolCall) -> anyhow::Result<serde_json::Value>;
}

/// Thread memory: short-term conversation stream.
/// Abstracts `ThreadRegistry`.
///
/// Production: wraps `ThreadRegistry`.
/// Mock: in-memory tracking with inspection fields.
pub trait ThreadMemory: Send {
    fn is_seeded(&self) -> bool;
    fn thread_history(&self, window: usize) -> Option<String>;
    fn append_turn(&mut self, agent: &str, input: &str, response: &str);
    fn mark_seeded(&mut self);
}

// ── TurnDeps: bundled dependencies ───────────────────────────────────

/// All dependencies `run_turn_loop` needs, bundled for ergonomic passing.
///
/// 4 traits (behavioral, multi-method) + 3 closures (one-call dependencies).
/// Each is independently mockable. The struct has 7 fields — at the
/// essentialist G2 interface limit.
pub struct TurnDeps<'a> {
    pub executor: &'a dyn TurnExecutor,
    pub gas: &'a dyn GasGovernor,
    pub tools: &'a dyn ToolInvoker,
    pub threads: &'a mut dyn ThreadMemory,
    /// Build a TurnRequest from iteration inputs.
    pub build_request: &'a dyn Fn(&str, usize, Option<String>, Option<&str>) -> TurnRequest,
    /// CNS regulation tick + alert check. Called once after the loop.
    pub on_cns_update: &'a dyn Fn(),
    /// Speak a response via talk mode. Called for final responses.
    pub on_speak: &'a dyn Fn(&str),
}

// ── Production adapters ──────────────────────────────────────────────

/// Adapts `ChatService::execute_turn` to the `TurnExecutor` trait.
pub struct ReplTurnExecutor {
    ctx: Arc<AgentService>,
    manifest_executor: Option<hkask_templates::ManifestExecutor>,
    manifest: Option<hkask_templates::BundleManifest>,
}

impl ReplTurnExecutor {
    pub fn new(
        ctx: Arc<AgentService>,
        manifest_executor: Option<hkask_templates::ManifestExecutor>,
        manifest: Option<hkask_templates::BundleManifest>,
    ) -> Self {
        Self {
            ctx,
            manifest_executor,
            manifest,
        }
    }
}

#[async_trait::async_trait]
impl TurnExecutor for ReplTurnExecutor {
    async fn execute_turn(&self, req: &TurnRequest) -> Result<TurnResult, ServiceError> {
        ChatService::execute_turn(
            &self.ctx,
            req,
            self.manifest_executor.as_ref(),
            self.manifest.as_ref(),
        )
        .await
    }
}

/// Adapts `EnergyGuard` + gas status to the `GasGovernor` trait.
pub struct ReplGasGovernor {
    cybernetics_loop: Arc<tokio::sync::RwLock<hkask_cns::CyberneticsLoop>>,
    inference_loop: Arc<hkask_agents::InferenceLoop>,
    webid: WebID,
    rt: tokio::runtime::Handle,
    gas_remaining: fn(&AgentService) -> Option<u64>,
    gas_cap: fn(&AgentService) -> Option<u64>,
    ctx: Arc<AgentService>,
}

impl ReplGasGovernor {
    pub fn new(ctx: Arc<AgentService>, webid: WebID, rt: tokio::runtime::Handle) -> Self {
        Self {
            cybernetics_loop: ctx.cns().cybernetics.clone(),
            inference_loop: ctx.inference_loop().expect("inference loop").clone(),
            webid,
            rt,
            gas_remaining: |c| c.gas_remaining(),
            gas_cap: |c| c.gas_cap(),
            ctx,
        }
    }
}

impl GasGovernor for ReplGasGovernor {
    fn try_reserve(&self, heuristic: u64) -> Option<Box<dyn GasReservation>> {
        EnergyGuard::try_reserve(
            &self.cybernetics_loop,
            &self.inference_loop,
            &self.webid,
            &self.rt,
            heuristic,
        )
        .map(|guard| Box::new(ReplGasReservation { guard: Some(guard) }) as Box<dyn GasReservation>)
    }

    fn gas_status(&self) -> (u64, u64) {
        let remaining = (self.gas_remaining)(&self.ctx).unwrap_or(0);
        let cap = (self.gas_cap)(&self.ctx).unwrap_or(0);
        (remaining, cap)
    }
}

/// Adapts `EnergyGuard` to the `GasReservation` trait.
/// Uses `Option::take()` to consume the guard from behind `&mut self`.
struct ReplGasReservation {
    guard: Option<EnergyGuard>,
}

impl GasReservation for ReplGasReservation {
    fn heuristic(&self) -> u64 {
        self.guard.as_ref().map(|g| g.heuristic()).unwrap_or(0)
    }

    fn settle(&mut self, actual: u64) {
        if let Some(guard) = self.guard.take() {
            guard.settle(actual);
        }
    }

    fn release(&mut self) {
        if let Some(guard) = self.guard.take() {
            guard.release();
        }
    }
}

/// Adapts `GovernedTool` + token minting to the `ToolInvoker` trait.
pub struct ReplToolInvoker {
    governed_tool: Arc<GovernedTool<RawMcpToolPort>>,
    agent_webid: WebID,
    a2a_secret: Vec<u8>,
    host: Arc<dyn super::host::ReplHost>,
}

impl ReplToolInvoker {
    pub fn new(
        governed_tool: Arc<GovernedTool<RawMcpToolPort>>,
        agent_webid: WebID,
        a2a_secret: Vec<u8>,
        host: Arc<dyn super::host::ReplHost>,
    ) -> Self {
        Self {
            governed_tool,
            agent_webid,
            a2a_secret,
            host,
        }
    }
}

#[async_trait::async_trait]
impl ToolInvoker for ReplToolInvoker {
    async fn invoke(&self, call: &ToolCall) -> anyhow::Result<serde_json::Value> {
        super::tool_augmented::invoke_tool_call(
            call,
            &self.governed_tool,
            &self.agent_webid,
            &self.a2a_secret,
            self.host.as_ref(),
        )
        .await
    }
}

/// Adapts `ThreadRegistry` to the `ThreadMemory` trait.
pub struct ReplThreadMemory<'a> {
    registry: &'a mut ThreadRegistry,
}

impl<'a> ReplThreadMemory<'a> {
    pub fn new(registry: &'a mut ThreadRegistry) -> Self {
        Self { registry }
    }
}

impl<'a> ThreadMemory for ReplThreadMemory<'a> {
    fn is_seeded(&self) -> bool {
        self.registry.seeded
    }

    fn thread_history(&self, window: usize) -> Option<String> {
        self.registry.thread_history(Some(window))
    }

    fn append_turn(&mut self, agent: &str, input: &str, response: &str) {
        self.registry.append_turn(agent, input, response);
    }

    fn mark_seeded(&mut self) {
        self.registry.mark_seeded();
    }
}
