//! Storage context — template registry, goal repository, spec store,
//! agent registry, user store, sovereignty boundaries, and wallet store.
//!
//! Extracted from `AgentService` as part of the strangler-fig decomposition.

use hkask_storage::goals::SqliteGoalRepository;
use hkask_storage::user_store::UserStore;
use hkask_storage::{AgentRegistryStore, SovereigntyBoundaryStore, SqliteSpecStore, WalletStore};
use hkask_templates::SqliteRegistry;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Consolidated storage context — all persistent stores in one place.
pub struct StorageContext {
    pub registry: Arc<Mutex<SqliteRegistry>>,
    pub goals: Arc<SqliteGoalRepository>,
    pub specs: SqliteSpecStore,
    pub agents: AgentRegistryStore,
    pub users: Arc<std::sync::Mutex<UserStore>>,
    pub sovereignty: SovereigntyBoundaryStore,
    pub wallet: Option<Arc<WalletStore>>,
}

impl StorageContext {
    pub fn new(
        registry: Arc<Mutex<SqliteRegistry>>,
        goals: Arc<SqliteGoalRepository>,
        specs: SqliteSpecStore,
        agents: AgentRegistryStore,
        users: Arc<std::sync::Mutex<UserStore>>,
        sovereignty: SovereigntyBoundaryStore,
        wallet: Option<Arc<WalletStore>>,
    ) -> Self {
        Self {
            registry,
            goals,
            specs,
            agents,
            users,
            sovereignty,
            wallet,
        }
    }
}
