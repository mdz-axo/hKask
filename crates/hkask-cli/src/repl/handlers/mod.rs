//! REPL sub-handler modules — one file per slash command domain

pub(crate) mod agent;
pub(crate) mod ask;
pub(crate) mod consolidation;
pub(crate) mod ensemble;
pub(crate) mod escalation;
pub(crate) mod hhh;
pub(crate) mod info;
pub(crate) mod invoke;
pub(crate) mod model;
pub(crate) mod repl_settings;
pub(crate) mod status;

pub(crate) use agent::{handle_agent, handle_agents};
pub(crate) use ask::handle_ask;
pub(crate) use consolidation::handle_consolidate;
pub(crate) use ensemble::{handle_ensemble, handle_filter, handle_into, handle_mode};
pub(crate) use escalation::{handle_dismiss, handle_escalations, handle_resolve};
pub(crate) use hhh::handle_hhh;
pub(crate) use info::{handle_history, handle_pods, handle_templates, handle_tools};
pub(crate) use invoke::handle_invoke;
pub(crate) use model::handle_model;
pub(crate) use repl_settings::{ReplSettings, handle_repl_set, to_llm_params};
pub(crate) use status::handle_status;
