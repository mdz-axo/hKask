//! Register built-in MCP servers at REPL startup.
//!
//! The REPL starts with an empty `McpRuntime`. This module registers the core
//! MCP servers (inference, cns, condenser, episodic, semantic, ocap, keystore,
//! git, registry, goal) so that `/tools` and `/invoke` have something to work
//! with immediately.
//!
//! These are **metadata registrations only** — the actual MCP server processes
//! run out-of-process and are connected via transport when invoked. Registering
//! them here makes their tools discoverable and invocable through GovernedTool.

use hkask_mcp::runtime::{McpRuntime, McpServer, McpTool};

/// Register all built-in MCP servers with the given runtime.
///
/// Returns the number of tools registered across all servers.
pub async fn register_builtin_servers(runtime: &McpRuntime) -> usize {
    let mut total_tools = 0;

    // ── Inference ──────────────────────────────────────────────────────────
    let inference = McpServer {
        id: "hkask-mcp-inference".into(),
        name: "Inference".into(),
        tools: vec![
            McpTool {
                name: "inference_generate".into(),
                description: "Generate text using Okapi-backed LLM inference. Supports model selection with automatic failover, temperature control, and token limits.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "prompt": { "type": "string", "description": "The prompt to generate from" },
                        "model": { "type": "string", "default": "ollama/llama-3.1-8b-instruct", "description": "Model to use" },
                        "fallback_model": { "type": "string", "default": "", "description": "Fallback model if primary fails" },
                        "temperature": { "type": "number", "default": 0.7 },
                        "max_tokens": { "type": "integer", "default": 1024 },
                        "caller_id": { "type": "string", "default": "anonymous" }
                    },
                    "required": ["prompt"]
                }),
                server_id: "hkask-mcp-inference".into(),
            },
            McpTool {
                name: "inference_metrics".into(),
                description: "Get current inference metrics including total requests, tokens generated, error counts, and failover count.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "reset": { "type": "boolean", "default": false, "description": "Reset counters after reading" }
                    }
                }),
                server_id: "hkask-mcp-inference".into(),
            },
            McpTool {
                name: "inference_models".into(),
                description: "List available model tiers and their configurations.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "filter": { "type": "string", "default": "", "description": "Filter models by name substring" }
                    }
                }),
                server_id: "hkask-mcp-inference".into(),
            },
        ],
    };
    total_tools += inference.tools.len();
    runtime.register_server(inference).await;

    // ── CNS (Cybernetic Nervous System) ────────────────────────────────────
    let cns = McpServer {
        id: "hkask-mcp-cns".into(),
        name: "CNS".into(),
        tools: vec![
            McpTool {
                name: "cns_emit".into(),
                description: "Emit a CNS observation event.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "span": { "type": "string" },
                        "observer_webid": { "type": "string" },
                        "phase": { "type": "string" },
                        "observation": { "type": "string" }
                    },
                    "required": ["span", "observation"]
                }),
                server_id: "hkask-mcp-cns".into(),
            },
            McpTool {
                name: "cns_variety".into(),
                description: "Get variety count for a span pattern.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "span_pattern": { "type": "string" }
                    },
                    "required": ["span_pattern"]
                }),
                server_id: "hkask-mcp-cns".into(),
            },
            McpTool {
                name: "cns_alert".into(),
                description: "Trigger an algedonic alert.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "span_pattern": { "type": "string" },
                        "severity": { "type": "string" }
                    },
                    "required": ["span_pattern"]
                }),
                server_id: "hkask-mcp-cns".into(),
            },
            McpTool {
                name: "cns_calibrate".into(),
                description: "Calibrate a span threshold.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "span_pattern": { "type": "string" },
                        "new_threshold": { "type": "integer" }
                    },
                    "required": ["span_pattern", "new_threshold"]
                }),
                server_id: "hkask-mcp-cns".into(),
            },
            McpTool {
                name: "cns_list_alerts".into(),
                description: "List active algedonic alerts.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer" }
                    }
                }),
                server_id: "hkask-mcp-cns".into(),
            },
            McpTool {
                name: "cns_health".into(),
                description: "Get CNS health status.".into(),
                input_schema: serde_json::json!({ "type": "object" }),
                server_id: "hkask-mcp-cns".into(),
            },
        ],
    };
    total_tools += cns.tools.len();
    runtime.register_server(cns).await;

    // ── Condenser ───────────────────────────────────────────────────────────
    let condenser = McpServer {
        id: "hkask-mcp-condenser".into(),
        name: "Condenser".into(),
        tools: vec![
            McpTool {
                name: "condenser_ping".into(),
                description: "Liveness and profile info.".into(),
                input_schema: serde_json::json!({ "type": "object" }),
                server_id: "hkask-mcp-condenser".into(),
            },
            McpTool {
                name: "condenser_compress".into(),
                description: "Compress tool output using context-aware algorithms.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "tool_name": { "type": "string" },
                        "output": { "type": "string" },
                        "category": { "type": "string" }
                    },
                    "required": ["tool_name", "output"]
                }),
                server_id: "hkask-mcp-condenser".into(),
            },
            McpTool {
                name: "condenser_set_profile".into(),
                description: "Set compression profile (heavy/normal/soft/light).".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "profile": { "type": "string" }
                    },
                    "required": ["profile"]
                }),
                server_id: "hkask-mcp-condenser".into(),
            },
            McpTool {
                name: "condenser_stats".into(),
                description: "Cumulative compression statistics.".into(),
                input_schema: serde_json::json!({ "type": "object" }),
                server_id: "hkask-mcp-condenser".into(),
            },
            McpTool {
                name: "condenser_classify".into(),
                description: "Classify tool name to context category.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "tool_name": { "type": "string" }
                    },
                    "required": ["tool_name"]
                }),
                server_id: "hkask-mcp-condenser".into(),
            },
            McpTool {
                name: "condenser_persist".into(),
                description: "Persist a compressed output to episodic memory.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "tool_name": { "type": "string" },
                        "compressed_output": { "type": "string" },
                        "confidence": { "type": "number" }
                    },
                    "required": ["tool_name", "compressed_output"]
                }),
                server_id: "hkask-mcp-condenser".into(),
            },
        ],
    };
    total_tools += condenser.tools.len();
    runtime.register_server(condenser).await;

    // ── Episodic Memory ─────────────────────────────────────────────────────
    let episodic = McpServer {
        id: "hkask-mcp-episodic".into(),
        name: "Episodic".into(),
        tools: vec![
            McpTool {
                name: "episodic_ping".into(),
                description: "Liveness and storage info for episodic memory.".into(),
                input_schema: serde_json::json!({ "type": "object" }),
                server_id: "hkask-mcp-episodic".into(),
            },
            McpTool {
                name: "episodic_store".into(),
                description: "Store an episodic triple (private, perspective-bound).".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "entity": { "type": "string" },
                        "attribute": { "type": "string" },
                        "value": { "type": "object" },
                        "confidence": { "type": "number" }
                    },
                    "required": ["entity", "attribute", "value"]
                }),
                server_id: "hkask-mcp-episodic".into(),
            },
            McpTool {
                name: "episodic_recall".into(),
                description: "Recall episodic triples by entity.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "entity": { "type": "string" }
                    },
                    "required": ["entity"]
                }),
                server_id: "hkask-mcp-episodic".into(),
            },
            McpTool {
                name: "episodic_budget".into(),
                description: "Storage usage and budget for episodic memory.".into(),
                input_schema: serde_json::json!({ "type": "object" }),
                server_id: "hkask-mcp-episodic".into(),
            },
        ],
    };
    total_tools += episodic.tools.len();
    runtime.register_server(episodic).await;

    // ── Semantic Memory ──────────────────────────────────────────────────────
    let semantic = McpServer {
        id: "hkask-mcp-semantic".into(),
        name: "Semantic".into(),
        tools: vec![
            McpTool {
                name: "semantic_ping".into(),
                description: "Liveness and storage info for semantic memory.".into(),
                input_schema: serde_json::json!({ "type": "object" }),
                server_id: "hkask-mcp-semantic".into(),
            },
            McpTool {
                name: "semantic_store".into(),
                description: "Store a shared semantic triple.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "entity": { "type": "string" },
                        "attribute": { "type": "string" },
                        "value": { "type": "object" },
                        "confidence": { "type": "number" }
                    },
                    "required": ["entity", "attribute", "value"]
                }),
                server_id: "hkask-mcp-semantic".into(),
            },
            McpTool {
                name: "semantic_recall".into(),
                description: "Recall shared semantic triples by entity.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "entity": { "type": "string" }
                    },
                    "required": ["entity"]
                }),
                server_id: "hkask-mcp-semantic".into(),
            },
            McpTool {
                name: "semantic_embed".into(),
                description: "Store an embedding vector for similarity search.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "entity_ref": { "type": "string" },
                        "vector": { "type": "array", "items": { "type": "number" } },
                        "model": { "type": "string" }
                    },
                    "required": ["entity_ref", "vector", "model"]
                }),
                server_id: "hkask-mcp-semantic".into(),
            },
            McpTool {
                name: "semantic_search".into(),
                description: "KNN similarity search over embeddings.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query_vector": { "type": "array", "items": { "type": "number" } },
                        "limit": { "type": "integer" }
                    },
                    "required": ["query_vector"]
                }),
                server_id: "hkask-mcp-semantic".into(),
            },
            McpTool {
                name: "semantic_count".into(),
                description: "Triple and embedding counts for semantic memory.".into(),
                input_schema: serde_json::json!({ "type": "object" }),
                server_id: "hkask-mcp-semantic".into(),
            },
            McpTool {
                name: "semantic_centroid".into(),
                description:
                    "Compute mean embedding vector (centroid) for embeddings matching a prefix."
                        .into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "prefix": { "type": "string" },
                        "exclude_prefix": { "type": "string" },
                        "exclude_ref": { "type": "string" },
                        "dim": { "type": "integer" }
                    },
                    "required": ["prefix", "exclude_prefix", "exclude_ref", "dim"]
                }),
                server_id: "hkask-mcp-semantic".into(),
            },
            McpTool {
                name: "semantic_purge".into(),
                description: "Delete all embeddings whose entity_ref starts with a prefix.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "prefix": { "type": "string" },
                        "dim": { "type": "integer" }
                    },
                    "required": ["prefix", "dim"]
                }),
                server_id: "hkask-mcp-semantic".into(),
            },
        ],
    };
    total_tools += semantic.tools.len();
    runtime.register_server(semantic).await;

    // ── OCAP (Capability) ──────────────────────────────────────────────────
    let ocap = McpServer {
        id: "hkask-mcp-ocap".into(),
        name: "OCAP".into(),
        tools: vec![
            McpTool {
                name: "ocap_delegate".into(),
                description: "Create a delegated capability token with HMAC signature.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "issuer": { "type": "string" },
                        "subject": { "type": "string" },
                        "capabilities": { "type": "string" }
                    },
                    "required": ["issuer", "subject", "capabilities"]
                }),
                server_id: "hkask-mcp-ocap".into(),
            },
            McpTool {
                name: "ocap_verify".into(),
                description: "Verify a capability token with cryptographic HMAC verification."
                    .into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "token_id": { "type": "string" },
                        "capability": { "type": "string" }
                    },
                    "required": ["token_id", "capability"]
                }),
                server_id: "hkask-mcp-ocap".into(),
            },
            McpTool {
                name: "ocap_revoke".into(),
                description: "Revoke a capability token.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "token_id": { "type": "string" }
                    },
                    "required": ["token_id"]
                }),
                server_id: "hkask-mcp-ocap".into(),
            },
            McpTool {
                name: "ocap_enumerate".into(),
                description: "Enumerate capabilities for a subject.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "subject": { "type": "string" }
                    },
                    "required": ["subject"]
                }),
                server_id: "hkask-mcp-ocap".into(),
            },
            McpTool {
                name: "ocap_list_tokens".into(),
                description: "List all capability tokens.".into(),
                input_schema: serde_json::json!({ "type": "object" }),
                server_id: "hkask-mcp-ocap".into(),
            },
        ],
    };
    total_tools += ocap.tools.len();
    runtime.register_server(ocap).await;

    // ── Keystore ────────────────────────────────────────────────────────────
    let keystore = McpServer {
        id: "hkask-mcp-keystore".into(),
        name: "Keystore".into(),
        tools: vec![
            McpTool {
                name: "keystore_set".into(),
                description: "Set a key-value pair in the keystore with AES-256-GCM encryption."
                    .into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "key": { "type": "string" },
                        "value": { "type": "string" },
                        "service": { "type": "string" },
                        "owner_webid": { "type": "string" }
                    },
                    "required": ["key", "value"]
                }),
                server_id: "hkask-mcp-keystore".into(),
            },
            McpTool {
                name: "keystore_get".into(),
                description: "Get a value from the keystore (capability-gated).".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "key": { "type": "string" },
                        "service": { "type": "string" },
                        "caller_webid": { "type": "string" }
                    },
                    "required": ["key"]
                }),
                server_id: "hkask-mcp-keystore".into(),
            },
            McpTool {
                name: "keystore_rotate".into(),
                description: "Rotate a key-value pair with re-encryption.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "key": { "type": "string" },
                        "new_value": { "type": "string" },
                        "service": { "type": "string" },
                        "caller_webid": { "type": "string" }
                    },
                    "required": ["key", "new_value"]
                }),
                server_id: "hkask-mcp-keystore".into(),
            },
            McpTool {
                name: "keystore_delete".into(),
                description: "Delete a key from the keystore (capability-gated).".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "key": { "type": "string" },
                        "service": { "type": "string" },
                        "caller_webid": { "type": "string" }
                    },
                    "required": ["key"]
                }),
                server_id: "hkask-mcp-keystore".into(),
            },
            McpTool {
                name: "keystore_list".into(),
                description: "List all keys in the keystore.".into(),
                input_schema: serde_json::json!({ "type": "object" }),
                server_id: "hkask-mcp-keystore".into(),
            },
        ],
    };
    total_tools += keystore.tools.len();
    runtime.register_server(keystore).await;

    // ── Git ─────────────────────────────────────────────────────────────────
    let git = McpServer {
        id: "hkask-mcp-git".into(),
        name: "Git".into(),
        tools: vec![
            McpTool {
                name: "git_resolve".into(),
                description: "Resolve a git reference to a SHA.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "git_ref": { "type": "string" }
                    },
                    "required": ["git_ref"]
                }),
                server_id: "hkask-mcp-git".into(),
            },
            McpTool {
                name: "git_snapshot".into(),
                description: "Create a git snapshot (commit).".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "message": { "type": "string" },
                        "branch": { "type": "string" }
                    },
                    "required": ["message"]
                }),
                server_id: "hkask-mcp-git".into(),
            },
            McpTool {
                name: "git_clone".into(),
                description: "Clone a git repository.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url": { "type": "string" },
                        "target_path": { "type": "string" },
                        "branch": { "type": "string" }
                    },
                    "required": ["url", "target_path"]
                }),
                server_id: "hkask-mcp-git".into(),
            },
            McpTool {
                name: "git_diff".into(),
                description: "Show diff between two commits.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "sha1": { "type": "string" },
                        "sha2": { "type": "string" },
                        "path": { "type": "string" }
                    },
                    "required": ["sha1", "sha2"]
                }),
                server_id: "hkask-mcp-git".into(),
            },
            McpTool {
                name: "git_list".into(),
                description: "List files in a git path.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    }
                }),
                server_id: "hkask-mcp-git".into(),
            },
        ],
    };
    total_tools += git.tools.len();
    runtime.register_server(git).await;

    // ── Registry ────────────────────────────────────────────────────────────
    let registry = McpServer {
        id: "hkask-mcp-registry".into(),
        name: "Registry".into(),
        tools: vec![
            McpTool {
                name: "registry_index".into(),
                description: "Index templates from a root path.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "root_path": { "type": "string" },
                        "template_type": { "type": "string" }
                    },
                    "required": ["root_path"]
                }),
                server_id: "hkask-mcp-registry".into(),
            },
            McpTool {
                name: "registry_discover".into(),
                description: "Discover templates by type and domain.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "template_type": { "type": "string" },
                        "domain_hint": { "type": "string" },
                        "limit": { "type": "integer" }
                    }
                }),
                server_id: "hkask-mcp-registry".into(),
            },
            McpTool {
                name: "registry_validate".into(),
                description: "Validate a template by ID.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "template_id": { "type": "string" }
                    },
                    "required": ["template_id"]
                }),
                server_id: "hkask-mcp-registry".into(),
            },
            McpTool {
                name: "registry_reload".into(),
                description: "Reload templates from a path.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"]
                }),
                server_id: "hkask-mcp-registry".into(),
            },
            McpTool {
                name: "registry_compose".into(),
                description: "Compose templates with cascade.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "root_template_id": { "type": "string" },
                        "cascade_template_ids": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["root_template_id"]
                }),
                server_id: "hkask-mcp-registry".into(),
            },
            McpTool {
                name: "registry_get".into(),
                description: "Get a template by ID.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "template_id": { "type": "string" }
                    },
                    "required": ["template_id"]
                }),
                server_id: "hkask-mcp-registry".into(),
            },
        ],
    };
    total_tools += registry.tools.len();
    runtime.register_server(registry).await;

    // ── Goal ────────────────────────────────────────────────────────────────
    let goal = McpServer {
        id: "hkask-mcp-goal".into(),
        name: "Goal".into(),
        tools: vec![
            McpTool {
                name: "goal_create".into(),
                description: "Create a goal owned by the calling agent (OCAP-gated).".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "text": { "type": "string" },
                        "visibility": { "type": "string" }
                    },
                    "required": ["text"]
                }),
                server_id: "hkask-mcp-goal".into(),
            },
            McpTool {
                name: "goal_list".into(),
                description: "List the calling agent's goals, optionally filtered by state.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "state": { "type": "string" }
                    }
                }),
                server_id: "hkask-mcp-goal".into(),
            },
            McpTool {
                name: "goal_set_state".into(),
                description: "Transition a goal to a new state.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "goal_id": { "type": "string" },
                        "state": { "type": "string" }
                    },
                    "required": ["goal_id", "state"]
                }),
                server_id: "hkask-mcp-goal".into(),
            },
        ],
    };
    total_tools += goal.tools.len();
    runtime.register_server(goal).await;

    total_tools
}
