//! R7 Bot Identity — The seven bots that build and curate hKask
//!
//! Per personas-r7.md:
//!   "At launch, the only difference between them is which part of the
//!    hKask code each is responsible for. No personality differences.
//!    No capability differences. No memory differences. They share all
//!    registries. They can swap. They do swap."
//!
//! Memory visibility starts at Shared — the legion shares everything.
//! Divergence emerges through episodic memory, not design.

use crate::{Visibility, WebID};
use serde::{Deserialize, Serialize};

/// An R7 bot identity — one of the seven "c" curators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R7BotIdentity {
    /// Bot identifier (e.g., "R7.1")
    pub id: String,
    /// Display name
    pub name: String,
    /// Deterministic WebID derived from persona
    #[serde(skip)]
    pub webid: WebID,
    /// Primary hKask crate this bot is responsible for
    pub primary_crate: String,
    /// Bot description / persona fragment
    pub description: String,
    /// Energy budget for CNS tracking
    pub energy_budget: u64,
    /// Default memory visibility (starts as Shared for legion)
    pub memory_visibility: Visibility,
    /// Template domains this bot owns
    pub domains: Vec<String>,
}

impl R7BotIdentity {
    /// Derive the deterministic WebID for this bot
    pub fn derive_webid(id: &str) -> WebID {
        WebID::from_persona(id.as_bytes())
    }

    /// Compute the WebID after deserialization
    pub fn with_webid(mut self) -> Self {
        self.webid = Self::derive_webid(&self.id);
        self
    }
}

/// Container for the r7-bots.yaml file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R7BotRegistry {
    pub bots: Vec<R7BotIdentity>,
}

impl R7BotRegistry {
    /// Resolve WebIDs for all bots after deserialization
    pub fn resolve_webids(&mut self) {
        for bot in &mut self.bots {
            bot.webid = R7BotIdentity::derive_webid(&bot.id);
        }
    }

    /// Get a bot by its id
    pub fn get(&self, id: &str) -> Option<&R7BotIdentity> {
        self.bots.iter().find(|b| b.id == id)
    }

    /// Get the domains owned by a specific bot
    pub fn domains_for(&self, id: &str) -> Vec<String> {
        self.get(id).map(|b| b.domains.clone()).unwrap_or_default()
    }

    /// Get the union of all domains across all bots
    pub fn all_domains(&self) -> Vec<String> {
        let mut domains: Vec<String> = self.bots.iter().flat_map(|b| b.domains.clone()).collect();
        domains.sort();
        domains.dedup();
        domains
    }

    /// Find which bot owns a given domain
    pub fn owner_of(&self, domain: &str) -> Option<&R7BotIdentity> {
        self.bots
            .iter()
            .find(|b| b.domains.contains(&domain.to_string()))
    }
}

/// Default 7R7 bot definitions, embedded at compile time
///
/// This is the canonical 7R7 roster. Domain assignments can be changed
/// by the Curator at runtime, but the identity roster is fixed.
pub fn default_r7_bots() -> Vec<R7BotIdentity> {
    let bots = vec![
        (
            "R7.1",
            "hkask-storage",
            "Holds the data. The data must persist. The data must be encrypted. The data must be queryable.",
            10000,
            vec!["storage"],
        ),
        (
            "R7.2",
            "hkask-memory",
            "Holds the past. Semantic is public. Episodic is private. Knows the difference. Enforces OCAP.",
            10000,
            vec!["memory"],
        ),
        (
            "R7.3",
            "hkask-cns",
            "Holds the nervous system. Monitors variety. Sounds the alert when variety deficit >100.",
            10000,
            vec!["cns"],
        ),
        (
            "R7.4",
            "hkask-templates",
            "Holds the patterns. The registry is unified. The template_type discriminates.",
            10000,
            vec!["templates", "registry"],
        ),
        (
            "R7.5",
            "hkask-agents",
            "Holds the agents. Bots are public. Replicants are private or public. Curator is single.",
            8000,
            vec!["agents", "ensemble", "kata"],
        ),
        (
            "R7.6",
            "hkask-mcp",
            "Holds the tools. Fifteen MCP servers. Dispatches. Does not accumulate.",
            12000,
            vec![
                "mcp",
                "inference",
                "git",
                "web",
                "condenser",
                "github",
                "gml",
                "spec",
                "fmp",
                "telnyx",
                "fal",
                "rss-reader",
            ],
        ),
        (
            "R7.7",
            "hkask-cli",
            "Holds the interface. Humans need words. Gives them words. Does not meow at other bots.",
            8000,
            vec!["cli", "api"],
        ),
    ];

    bots.into_iter()
        .map(
            |(id, primary_crate, description, energy_budget, domains)| R7BotIdentity {
                webid: R7BotIdentity::derive_webid(id),
                id: id.to_string(),
                name: id.to_string(),
                primary_crate: primary_crate.to_string(),
                description: description.to_string(),
                energy_budget,
                memory_visibility: Visibility::Shared,
                domains: domains.into_iter().map(String::from).collect(),
            },
        )
        .collect()
}
