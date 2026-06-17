//! CLI commands implementation
//!
//! This module contains the actual command handlers, organized into focused submodules.
//! Each subcommand domain has its own module: agents, chat, curator, etc.

pub mod adapter;
pub mod agent;
pub mod backup_cmd;
pub mod bundle;
pub mod chat;
pub mod cns;
pub mod compose;

pub mod consolidation;
pub mod contract;
pub mod curator;
pub mod daemon;
pub mod discover;
pub mod docs;
pub mod embed_corpus;
pub mod git_cmd;
pub mod goal;
pub mod helpers;
pub mod kanban;
pub mod kata;
pub mod keystore;
pub mod loops;
pub mod magna_carta;
pub mod matrix;
pub mod mcp;

pub mod models;
pub mod onboard;
pub mod pod;
pub mod registry;
pub mod serve;
pub mod settings;
pub mod skill;
pub mod sovereignty;
pub mod spec;
pub mod style;
pub mod template;
pub mod test;
pub mod user;
pub mod wallet;
pub mod web_search;

// Re-exports from template
pub use template::{
    get_mcp_tool, get_template, list_mcp_servers, list_mcp_tools, list_templates,
    list_templates_local, register_mcp_server, register_template, search_templates,
};

// Re-exports from pod
pub use hkask_agents::pod::PodStatus;
pub use pod::{
    activate_pod, assign_role, create_pod, deactivate_pod, get_pod_status, list_pods, set_mode,
};

// Re-exports from agent
pub use agent::{AgentReceipt, agent_register, agent_unregister, bot_list, bot_status};

// Re-exports from chat
pub use chat::{
    ChatResponse, TokenUsage, chat_with_agent, chat_with_agent_streaming,
    chat_with_agent_streaming_with_params, chat_with_agent_with_params,
};

// Re-exports from curator
pub use curator::{curator_dismiss, curator_escalations, curator_metacognition, curator_resolve};

// Re-exports from bundle
pub use bundle::run_bundle;
