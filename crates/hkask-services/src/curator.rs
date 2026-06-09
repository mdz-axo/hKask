//! Curator service — escalation management and metacognition.
//!
//! `CuratorService` replaces the duplicated escalation queue operations
//! across CLI and API surfaces. Each surface constructs a `CuratorContext`
//! from its own state and delegates business logic to this service.
//!
//! # Design decisions
//!
//! - **Constraint: Prohibition (P1)** — MCP servers do NOT use this service.
//!   They continue using `EscalationQueue` directly because they run in
//!   separate processes and cannot share `CuratorContext`.
//! - **Constraint: Guideline** — `resolve_escalation` and `dismiss_escalation`
//!   verify existence before mutating. This normalizes behavior: the API
//!   currently checks, the CLI doesn't. Both surfaces will get the same
//!   `ServiceError::EscalationNotFound` error.
//! - **Depth test** — Deleting this module would cause escalation queue
//!   construction logic and existence checks to reappear in 8+ call sites.
//!   Passes deletion test.
//! - **Strangler fig** — `CuratorContext` is a lightweight struct that surfaces
//!   construct from their own state. For escalation-only operations, only
//!   `escalation_queue` is required. For `run_metacognition`, `cns_runtime`
//!   and `dispatch` are also required.
//! - **`run_metacognition`** constructs a fresh `CuratorAgent` each call.
//!   This matches the CLI's current behavior. The API doesn't run metacognition
//!   cycles — it only calls `escalation_stats()`. A shared MetacognitionLoop
//!   is a future Hypothesis.

use std::sync::Arc;

use hkask_agents::EscalationEntry;
use hkask_agents::EscalationQueue;
use hkask_agents::communication::MessageDispatch;
use hkask_agents::curator_agent::CuratorAgent;
use hkask_agents::escalation::EscalationStats;
use hkask_cns::CnsRuntime;
use hkask_types::CuratorHandle;

use crate::ServiceError;

/// Lightweight context for `CuratorService` calls.
///
/// Contains only the fields needed for escalation and metacognition
/// operations. Construct from surface state (CLI `ReplState`, API
/// `ApiState`) or from `ServiceContext` parts.
///
/// For escalation-only operations (list, get, resolve, dismiss, stats),
/// only `escalation_queue` is required — `cns_runtime` and `dispatch`
/// can be `None`.
///
/// For `run_metacognition`, `cns_runtime` and `dispatch` must be `Some`.
/// Calling `run_metacognition` without them returns
/// `ServiceError::Cns("...")`.
pub struct CuratorContext {
    /// Escalation queue — always required.
    pub escalation_queue: Arc<EscalationQueue>,
    /// CNS runtime — required for `run_metacognition`.
    pub cns_runtime: Option<Arc<CnsRuntime>>,
    /// Message dispatch — required for `run_metacognition`.
    pub dispatch: Option<Arc<MessageDispatch>>,
}

impl CuratorContext {
    /// Construct from individual parts.
    ///
    /// For escalation-only operations, pass `None` for `cns_runtime` and
    /// `dispatch`:
    /// ```ignore
    /// let ctx = CuratorContext::from_parts(escalation_queue, None, None);
    /// ```
    ///
    /// For metacognition operations, provide all three:
    /// ```ignore
    /// let ctx = CuratorContext::from_parts(
    ///     escalation_queue,
    ///     Some(cns_runtime),
    ///     Some(dispatch),
    /// );
    /// ```
    pub fn from_parts(
        escalation_queue: Arc<EscalationQueue>,
        cns_runtime: Option<Arc<CnsRuntime>>,
        dispatch: Option<Arc<MessageDispatch>>,
    ) -> Self {
        Self {
            escalation_queue,
            cns_runtime,
            dispatch,
        }
    }

    /// Construct a full CuratorContext from a ServiceContext (async).
    ///
    /// This method extracts `Arc<CnsRuntime>` from the `RwLock`-guarded
    /// `cns_runtime` field of `ServiceContext`. This requires an async read
    /// lock, so it cannot be a `From` impl.
    ///
    /// Use this for operations that need CNS runtime (e.g., `run_metacognition`).
    /// For escalation-only operations, use `CuratorContext::from(ctx)` which
    /// sets `cns_runtime: None`.
    pub async fn from_service_context(ctx: &crate::ServiceContext) -> Self {
        let cns_runtime = Some(Arc::new(ctx.cns_runtime.read().await.clone()));
        Self {
            escalation_queue: ctx.escalation_queue.clone(),
            cns_runtime,
            dispatch: Some(ctx.dispatch.clone()),
        }
    }
}

impl From<&crate::ServiceContext> for CuratorContext {
    /// Construct an escalation-only CuratorContext from a ServiceContext.
    ///
    /// Sets `cns_runtime: None` and `dispatch: Some(...)`. Suitable for
    /// escalation operations (list, get, resolve, dismiss, stats) but NOT
    /// for `run_metacognition` (which requires CNS runtime). For full context,
    /// use `CuratorContext::from_service_context(ctx).await`.
    fn from(ctx: &crate::ServiceContext) -> Self {
        Self {
            escalation_queue: ctx.escalation_queue.clone(),
            cns_runtime: None,
            dispatch: Some(ctx.dispatch.clone()),
        }
    }
}

/// Service-layer summary of a metacognition cycle.
///
/// Captures the public fields from `HealthSnapshot`. The `bot_status_reports`
/// field is `pub(crate)` in the domain crate, so this summary exposes
/// the structured data that surfaces can adapt (CLI prints `summary_text`,
/// API constructs JSON response).
#[derive(Debug)]
pub struct MetacognitionSummary {
    /// Human-readable summary text generated by the metacognition loop.
    pub summary_text: String,
    /// CNS health status string (e.g., "nominal", "degraded").
    pub cns_health: String,
    /// Variety counters per domain (domain name, current variety).
    pub variety_counters: Vec<(String, u64)>,
    /// Number of critical alerts in the system.
    pub critical_alerts: usize,
    /// Total number of alerts in the system.
    pub total_alerts: usize,
}

/// Curator service — escalation management and metacognition.
///
/// Use `CuratorService::list_escalations()` etc. to delegate escalation
/// operations through the service layer. Surfaces construct a
/// `CuratorContext` from their own state and call service methods.
pub struct CuratorService;

impl CuratorService {
    /// List all pending escalations.
    ///
    /// # REQ: svc-cur-001 — list_escalations returns all pending escalations
    pub fn list_escalations(ctx: &CuratorContext) -> Result<Vec<EscalationEntry>, ServiceError> {
        ctx.escalation_queue
            .list_pending()
            .map_err(ServiceError::from)
    }

    /// Get a specific escalation by ID.
    ///
    /// # REQ: svc-cur-002 — get_escalation returns escalation by ID or None
    pub fn get_escalation(
        ctx: &CuratorContext,
        id: &str,
    ) -> Result<Option<EscalationEntry>, ServiceError> {
        ctx.escalation_queue.get(id).map_err(ServiceError::from)
    }

    /// Resolve an escalation by ID.
    ///
    /// Verifies the escalation exists before resolving. Returns
    /// `ServiceError::EscalationNotFound` if the escalation doesn't exist.
    ///
    /// # REQ: svc-cur-003 — resolve_escalation verifies existence then resolves
    pub fn resolve_escalation(
        ctx: &CuratorContext,
        id: &str,
        resolved_by: &str,
    ) -> Result<(), ServiceError> {
        let entry = ctx.escalation_queue.get(id)?;
        if entry.is_none() {
            return Err(ServiceError::EscalationNotFound(id.to_string()));
        }
        ctx.escalation_queue
            .resolve(id, resolved_by)
            .map_err(ServiceError::from)
    }

    /// Dismiss an escalation by ID.
    ///
    /// Verifies the escalation exists before dismissing. Returns
    /// `ServiceError::EscalationNotFound` if the escalation doesn't exist.
    ///
    /// # REQ: svc-cur-004 — dismiss_escalation verifies existence then dismisses
    pub fn dismiss_escalation(
        ctx: &CuratorContext,
        id: &str,
        dismissed_by: &str,
    ) -> Result<(), ServiceError> {
        let entry = ctx.escalation_queue.get(id)?;
        if entry.is_none() {
            return Err(ServiceError::EscalationNotFound(id.to_string()));
        }
        ctx.escalation_queue
            .dismiss(id, dismissed_by)
            .map_err(ServiceError::from)
    }

    /// Get aggregated escalation statistics.
    ///
    /// # REQ: svc-cur-005 — escalation_stats returns total/pending/resolved/dismissed counts
    pub fn escalation_stats(ctx: &CuratorContext) -> Result<EscalationStats, ServiceError> {
        ctx.escalation_queue.stats().map_err(ServiceError::from)
    }

    /// Run a metacognition cycle and return a summary.
    ///
    /// Requires `cns_runtime` and `dispatch` in the `CuratorContext`.
    /// Returns `ServiceError::Cns` if either is missing.
    ///
    /// # REQ: svc-cur-006 — run_metacognition executes a full cycle and returns summary
    pub async fn run_metacognition(
        ctx: &CuratorContext,
    ) -> Result<MetacognitionSummary, ServiceError> {
        let cns = ctx.cns_runtime.as_ref().ok_or_else(|| {
            ServiceError::Cns("CNS runtime required for metacognition".to_string())
        })?;
        let dispatch = ctx.dispatch.as_ref().ok_or_else(|| {
            ServiceError::Cns("Message dispatch required for metacognition".to_string())
        })?;

        let curator_handle = CuratorHandle::system();
        let agents_context = Arc::new(hkask_agents::CuratorContext::new(
            curator_handle,
            Arc::clone(cns),
            Arc::clone(dispatch),
            Arc::clone(&ctx.escalation_queue),
        ));
        let agent = CuratorAgent::new(agents_context);
        let metacognition = agent.metacognition();
        let snapshot = metacognition.run_cycle().await?;

        Ok(MetacognitionSummary {
            summary_text: metacognition.generate_summary(&snapshot),
            cns_health: snapshot.cns_health,
            variety_counters: snapshot.variety_counters,
            critical_alerts: snapshot.critical_alerts,
            total_alerts: snapshot.total_alerts,
        })
    }
}

