//! REPL sub-handler modules — one file per slash command domain

pub(crate) mod ensemble;
pub(crate) mod into;
pub(crate) mod model;

pub(crate) use ensemble::handle_ensemble;
pub(crate) use into::handle_into;
pub(crate) use model::handle_model;
