//! CLI commands implementation
//!
//! This module contains the actual command handlers, organized into focused submodules.

pub mod admin;
pub mod agent;
pub mod config;
pub mod factories;
pub mod pod;
pub mod russell;
pub mod template_mcp;
pub mod user;

// Git archival commands (Phase 9)
pub use super::git_archival::{
    archive_registry_to_git, create_registry_snapshot, list_registry_archives,
    restore_registry_from_git,
};

// Re-exports from template_mcp
pub use template_mcp::{
    get_mcp_tool, get_template, list_mcp_servers, list_mcp_tools, list_templates,
    list_templates_local, register_mcp_server, register_template, search_templates,
};

// Re-exports from pod
pub use pod::{PodStatus, activate_pod, create_pod, deactivate_pod, get_pod_status, list_pods};

// Re-exports from russell
pub use russell::{import_russell, import_russell_with_mapper};

// Re-exports from factories
pub use factories::{
    ensemble_chat_create, ensemble_chat_list, ensemble_chat_register, ensemble_chat_send,
    ensemble_deliberation_create, ensemble_deliberation_list, ensemble_deliberation_record,
    ensemble_deliberation_start, ensemble_deliberation_synthesize, ensemble_improv_config,
    ensemble_improv_set_mode, ensemble_improv_set_threshold, ensemble_improv_turn,
    ensemble_participants,
};

// Re-exports from agent
pub use agent::{
    AgentReceipt, agent_register, agent_unregister, bot_list, bot_status, chat_with_agent,
    curator_dismiss, curator_escalations, curator_metacognition, curator_resolve,
    ensemble_standing_start, ensemble_standing_status,
};
