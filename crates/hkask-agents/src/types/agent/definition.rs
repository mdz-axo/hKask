//! Agent Definition types — re-exported canonical schema from hkask-types.

pub use hkask_types::agent_registry::{AgentDefinition, Charter, RegisteredAgent};
// AgentKind is defined in hkask-types (canonical, with SQL impls). Re-exported.
pub use hkask_types::AgentKind;
// PersonaConstraints is defined in hkask-types. Re-exported.
pub use hkask_types::PersonaConstraints;
