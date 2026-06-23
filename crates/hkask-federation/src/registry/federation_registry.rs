//! FederationRegistry — merged user/agent resolution across federated servers.
//!
//! Local users/agents are authoritative on their home server. Remote entries
//! arrive via CRDT sync and are merged. Resolution checks local first,
//! then falls back to the CRDT-merged remote view.

use std::collections::HashMap;

use crate::crdt::{GSet, LWWMap};

/// Simplified user profile for federation exchange.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FederatedUserProfile {
    pub webid: String,
    pub display_name: String,
    pub matrix_id: Option<String>,
    pub home_server: String,
}

/// Simplified agent entry for federation exchange.
#[derive(Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FederatedAgentEntry {
    pub webid: String,
    pub agent_type: String,
    pub display_name: String,
    pub home_server: String,
}

/// Merged user/agent registry across the federation.
pub struct FederationRegistry {
    /// Local users — authoritative on this server.
    local_users: HashMap<String, FederatedUserProfile>,
    /// Local agents — authoritative on this server.
    local_agents: Vec<FederatedAgentEntry>,
    /// CRDT: remote users replicated from peers (LWW — latest timestamp wins).
    remote_users: LWWMap<String, FederatedUserProfile>,
    /// CRDT: remote agents replicated from peers (G-Set — additive).
    remote_agents: GSet<FederatedAgentEntry>,
}

impl FederationRegistry {
    /// Create a new registry.
    pub fn new() -> Self {
        Self {
            local_users: HashMap::new(),
            local_agents: Vec::new(),
            remote_users: LWWMap::new(),
            remote_agents: GSet::new(),
        }
    }

    /// Register a local user (authoritative on this server).
    pub fn register_local_user(&mut self, profile: FederatedUserProfile) {
        self.local_users.insert(profile.webid.clone(), profile);
    }

    /// Register a local agent (authoritative on this server).
    pub fn register_local_agent(&mut self, entry: FederatedAgentEntry) {
        self.local_agents.push(entry);
    }

    /// Merge remote user profiles from a federation sync.
    pub fn merge_remote_users(&mut self, remote: &LWWMap<String, FederatedUserProfile>) {
        self.remote_users.merge(remote);
    }

    /// Merge remote agent entries from a federation sync.
    pub fn merge_remote_agents(&mut self, remote: &GSet<FederatedAgentEntry>) {
        self.remote_agents.merge(remote);
    }

    /// Resolve a user by WebID. Local first, then remote.
    pub fn resolve_user(&self, webid: &str) -> Option<&FederatedUserProfile> {
        self.local_users
            .get(webid)
            .or_else(|| self.remote_users.get(&webid.to_string()))
    }

    /// Resolve an agent by WebID. Local first, then remote.
    pub fn resolve_agent(&self, webid: &str) -> Option<&FederatedAgentEntry> {
        self.local_agents
            .iter()
            .find(|a| a.webid == webid)
            .or_else(|| self.remote_agents.elements().find(|a| a.webid == webid))
    }

    /// List all known users (local + remote).
    pub fn all_users(&self) -> Vec<&FederatedUserProfile> {
        let mut all: Vec<&FederatedUserProfile> = self.local_users.values().collect();
        for (_k, v) in self.remote_users.entries() {
            all.push(v);
        }
        all
    }

    /// List all known agents (local + remote).
    pub fn all_agents(&self) -> Vec<&FederatedAgentEntry> {
        let mut all: Vec<&FederatedAgentEntry> = self.local_agents.iter().collect();
        for agent in self.remote_agents.elements() {
            all.push(agent);
        }
        all
    }

    /// Check if a WebID is local (authoritative on this server).
    pub fn is_local(&self, webid: &str) -> bool {
        self.local_users.contains_key(webid) || self.local_agents.iter().any(|a| a.webid == webid)
    }

    /// Get count of remote users.
    pub fn remote_user_count(&self) -> usize {
        self.remote_users.len()
    }

    /// Get count of remote agents.
    pub fn remote_agent_count(&self) -> usize {
        self.remote_agents.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_profile(webid: &str, home: &str) -> FederatedUserProfile {
        FederatedUserProfile {
            webid: webid.into(),
            display_name: format!("User {webid}"),
            matrix_id: Some(format!("@{webid}:{home}")),
            home_server: home.into(),
        }
    }

    fn make_agent(webid: &str, home: &str) -> FederatedAgentEntry {
        FederatedAgentEntry {
            webid: webid.into(),
            agent_type: "replicant".into(),
            display_name: format!("Agent {webid}"),
            home_server: home.into(),
        }
    }

    #[test]
    fn resolve_local_user_first() {
        let mut reg = FederationRegistry::new();
        reg.register_local_user(make_profile("user1", "a.example.com"));

        assert!(reg.resolve_user("user1").is_some());
        assert!(reg.is_local("user1"));
    }

    #[test]
    fn resolve_remote_user_fallback() {
        let mut reg = FederationRegistry::new();
        let mut remote: LWWMap<String, FederatedUserProfile> = LWWMap::new();
        remote.insert(
            "user2".into(),
            make_profile("user2", "b.example.com"),
            Utc::now(),
            "beta".into(),
        );
        reg.merge_remote_users(&remote);

        assert!(reg.resolve_user("user2").is_some());
        assert!(!reg.is_local("user2"));
    }

    #[test]
    fn resolve_remote_agent() {
        let mut reg = FederationRegistry::new();
        let mut remote: GSet<FederatedAgentEntry> = GSet::new();
        remote.insert(make_agent("bot1", "b.example.com"));
        reg.merge_remote_agents(&remote);

        assert!(reg.resolve_agent("bot1").is_some());
    }
}
