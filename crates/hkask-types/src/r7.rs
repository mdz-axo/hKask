//! R7 Bot Identity — Loop 4 (Communication): standing session coordination
//!
//! The R7 bots participate in standing sessions coordinated by
//! the Communication loop. Energy budgets are governed by Cybernetics;
//! the roster itself is Communication infrastructure.

use crate::{Visibility, WebID};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Loop: Communication
/// An R7 bot identity — one of the seven "c" curators
///
/// [DECLARATIVE] Invariant: `webid` is always `WebID::from_persona(id.as_bytes())`. (P8 — Semantic Grounding).
/// The `webid` field is computed on construction, not serialized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R7BotIdentity {
    /// Bot identifier (e.g., "R7.1")
    pub id: String,
    /// Display name
    pub name: String,
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
    // Invariant: always WebID::from_persona(id.as_bytes())
    #[serde(skip)]
    webid: WebID,
}

impl R7BotIdentity {
    /// Derive the deterministic WebID for a bot id
    pub(crate) fn derive_webid(id: &str) -> WebID {
        WebID::from_persona(id.as_bytes())
    }

    /// Get the bot's WebID (always valid by construction)
    ///
    /// REQ: TYP-186
    /// pre:  self is a valid R7BotIdentity (constructed via new or deserialized
    ///       with a valid webid field)
    /// post: returns the deterministic WebID derived from the bot's id at
    ///       construction time
    pub fn webid(&self) -> WebID {
        self.webid
    }

    /// Construct a new R7BotIdentity. Sets webid deterministically.
    pub(crate) fn new(
        id: String,
        primary_crate: String,
        description: String,
        energy_budget: u64,
        domains: Vec<String>,
    ) -> Self {
        let webid = Self::derive_webid(&id);
        Self {
            webid,
            id: id.clone(),
            name: id,
            primary_crate,
            description,
            energy_budget,
            memory_visibility: Visibility::Public,
            domains,
        }
    }
}

/// Cached default R7 bot definitions — allocated once, shared everywhere.
static DEFAULT_R7_BOTS: OnceLock<Vec<R7BotIdentity>> = OnceLock::new();

/// Default 7R7 bot definitions, embedded at compile time.
///
/// This is the canonical 7R7 roster. Domain assignments can be changed
/// by the Curator at runtime, but the identity roster is fixed.
///
/// Returns a reference to a statically cached `Vec<R7BotIdentity>`.
///
/// REQ: TYP-187
/// pre:  (none — always callable)
/// post: returns a &'static [R7BotIdentity] slice of exactly 7 entries
///       (R7.1 through R7.7), cached after the first call via OnceLock
pub fn default_r7_bots() -> &'static [R7BotIdentity] {
    DEFAULT_R7_BOTS.get_or_init(|| {
        vec![
            R7BotIdentity::new(
                "R7.1".into(),
                "hkask-storage".into(),
                "Holds the data. The data must persist. The data must be encrypted. The data must be queryable.".into(),
                10000,
                vec!["storage".into()],
            ),
            R7BotIdentity::new(
                "R7.2".into(),
                "hkask-memory".into(),
                "Holds the past. Semantic is public. Episodic is private. Knows the difference. Enforces OCAP.".into(),
                10000,
                vec!["memory".into()],
            ),
            R7BotIdentity::new(
                "R7.3".into(),
                "hkask-cns".into(),
                "Holds the nervous system. Monitors variety. Sounds the alert when variety deficit >100. Watches the public seam — knows when behavioral guarantees are thinning.".into(),
                10000,
                vec!["cns".into(), "seam".into()],
            ),
            R7BotIdentity::new(
                "R7.4".into(),
                "hkask-templates".into(),
                "Holds the patterns. The registry is unified. The template_type discriminates.".into(),
                10000,
                vec!["templates".into(), "registry".into()],
            ),
            R7BotIdentity::new(
                "R7.5".into(),
                "hkask-agents".into(),
                "Holds the agents. Bots are public. Replicants are private or public. Curator is single.".into(),
                8000,
                vec!["agents".into(), "kata".into()],
            ),
            R7BotIdentity::new(
                "R7.6".into(),
                "hkask-mcp".into(),
                "Holds the tools. Eleven MCP servers. Dispatches. Does not accumulate.".into(),
                12000,
                vec![
                    "mcp".into(), "web".into(),
                    "condenser".into(), "spec".into(),
                    "memory".into(), "docproc".into(), "training".into(),
                    "companies".into(), "fal".into(), "rss-reader".into(),
                    "replica".into(),
                ],
            ),
            R7BotIdentity::new(
                "R7.7".into(),
                "hkask-cli".into(),
                "Holds the interface. Humans need words. Gives them words. Does not meow at other bots.".into(),
                8000,
                vec!["cli".into(), "api".into()],
            ),
        ]
    })
}
