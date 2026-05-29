//! REPL sub-handler modules — one file per slash command domain

pub(super) mod ensemble;
pub(super) mod into;
pub(super) mod model;

pub(super) use ensemble::handle_ensemble;
pub(super) use into::handle_into;
pub(super) use model::handle_model;
