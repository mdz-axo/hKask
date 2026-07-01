//! Handler trait for daemon queries — runs inside hKask.
//!
//! Implemented by the hKask runtime to provide authentication,
//! assignment verification, capability checking, and dual memory encoding.

/// Handler trait for daemon queries.
///
/// Implemented by the hKask runtime to provide authentication,
/// assignment verification, capability checking, and dual memory encoding.
#[async_trait::async_trait]
pub trait DaemonHandler: Send + Sync {
    /// Check if a replicant is authenticated. Returns (authenticated, webid).
    async fn check_auth(&self, replicant: &str) -> (bool, Option<String>);

    /// Check if a replicant is assigned to a role.
    async fn check_assignment(&self, replicant: &str, role: &str) -> bool;

    /// Check if a replicant holds a capability token for a tool.
    async fn check_capability(&self, replicant: &str, tool: &str) -> bool;

    /// Store an experience in both episodic and semantic memory.
    /// Returns (stored, episodic_triple_id, semantic_triple_id).
    async fn store_experience(
        &self,
        replicant: &str,
        entity: &str,
        attribute: &str,
        value: &serde_json::Value,
        confidence: Option<f64>,
    ) -> (bool, Option<String>, Option<String>);

    /// Dispatch a tool call to an MCP server.
    /// Returns (ok, output, error_message).
    async fn dispatch_tool(
        &self,
        replicant: &str,
        tool: &str,
        input: &serde_json::Value,
    ) -> (bool, Option<serde_json::Value>, Option<String>);

    /// Query curator system health — returns a HealthSnapshot as JSON.
    async fn curator_health(&self, replicant: &str) -> serde_json::Value;

    /// Query live CNS status — variety per domain, backpressure.
    async fn cns_status(&self, replicant: &str, domain: Option<&str>) -> serde_json::Value;

    /// Query spec drift — coherence evaluation and missing/extra verbs.
    async fn spec_drift(&self, _replicant: &str, _spec_id: Option<&str>) -> serde_json::Value {
        serde_json::json!({"status": "unavailable", "note": "spec_drift not available for this handler"})
    }
}
