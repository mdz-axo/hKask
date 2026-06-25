//! REPL sub-handler modules — one file per slash command domain

pub(crate) mod agent;
pub(crate) mod ask;
pub(crate) mod consolidation;
pub(crate) mod escalation;
pub(crate) mod feedback;
pub(crate) mod fusion;
pub(crate) mod improv;
pub(crate) mod info;
pub(crate) mod invoke;
pub(crate) mod listen;
#[cfg(feature = "communication")]
pub(crate) mod matrix;
pub(crate) mod mcp;
pub(crate) mod model;
pub(crate) mod repl_settings;
pub(crate) mod start;
pub(crate) mod status;

pub(crate) mod kanban;
pub(crate) mod talk;

pub(crate) use agent::{handle_agent, handle_agents};
pub(crate) use ask::handle_ask;
pub(crate) use consolidation::handle_consolidate;
pub(crate) use escalation::{handle_dismiss, handle_escalations, handle_resolve};
pub(crate) use feedback::handle_feedback;
pub(crate) use fusion::handle_fusion;
pub(crate) use improv::handle_improv;
pub(crate) use info::{handle_history, handle_pods, handle_templates, handle_tools};
pub(crate) use invoke::handle_invoke;
pub(crate) use kanban::handle_kanban;
pub(crate) use listen::handle_listen;
#[cfg(feature = "communication")]
pub(crate) use matrix::{handle_matrix, handle_msg};
pub(crate) use mcp::handle_mcp;
pub(crate) use model::handle_model;
pub(crate) use repl_settings::{ReplSettings, handle_repl_set, to_llm_params};
pub(crate) use start::handle_start;
pub(crate) use status::handle_status;
pub(crate) use talk::{handle_talk, speak_response};
