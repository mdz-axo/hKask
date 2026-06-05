//! REPL sub-handler modules — one file per slash command domain

pub(crate) mod consolidation;
pub(crate) mod ensemble;
pub(crate) mod into;
pub(crate) mod invoke;
pub(crate) mod model;

pub(crate) use consolidation::handle_consolidate;
pub(crate) use ensemble::handle_ensemble;
pub(crate) use into::handle_into;
pub(crate) use invoke::handle_invoke;
pub(crate) use model::handle_model;
