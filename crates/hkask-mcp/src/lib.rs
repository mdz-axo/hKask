//! hKask MCP — MCP runtime and governance
//!
//! Provides the McpRuntime for governed tool dispatch with OCAP,
//! energy budgeting, and cybernetic regulation. This is the heavy
//! runtime layer used by the REPL/API/CLI — MCP server binaries
//! depend on hkask-mcp-server instead.

pub mod runtime;

pub use runtime::{McpRuntime, McpServer, McpTool, ServerStartError};

// ── Canonical MCP server registry ─────────────────────────────────────────
pub const BUILTIN_SERVERS: &[(&str, &str)] = &[
    ("memory", "hkask-mcp-memory"),
    ("condenser", "hkask-mcp-condenser"),
    ("research", "hkask-mcp-research"),
    ("companies", "hkask-mcp-companies"),
    ("communication", "hkask-mcp-communication"),
    ("curator", "hkask-mcp-curator"),
    ("media", "hkask-mcp-media"),
    ("docproc", "hkask-mcp-docproc"),
    ("training", "hkask-mcp-training"),
    ("replica", "hkask-mcp-replica"),
    ("kanban", "hkask-mcp-kata-kanban"),
    ("skill", "hkask-mcp-skill"),
    ("filesystem", "hkask-mcp-filesystem"),
    ("codegraph", "hkask-mcp-codegraph"),
    ("scenarios", "hkask-mcp-scenarios"),
    ("regulation", "hkask-mcp-regulation"),
];
