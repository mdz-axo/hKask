//! ID types for hKask entities

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// WebID — Unique identifier for agents (bots and replicants)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WebID(pub Uuid);

impl WebID {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for WebID {
    fn default() -> Self {
        Self::new()
    }
}

/// TemplateID — Unique identifier for templates
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TemplateID(pub Uuid);

impl TemplateID {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TemplateID {
    fn default() -> Self {
        Self::new()
    }
}

/// BotID — Unique identifier for bots
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BotID(pub Uuid);

impl BotID {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for BotID {
    fn default() -> Self {
        Self::new()
    }
}

/// ManifestID — Unique identifier for manifests
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ManifestID(pub Uuid);

impl ManifestID {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ManifestID {
    fn default() -> Self {
        Self::new()
    }
}

/// TripleID — Unique identifier for bitemporal triples
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TripleID(pub Uuid);

impl TripleID {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TripleID {
    fn default() -> Self {
        Self::new()
    }
}

/// EventID — Unique identifier for ν-events
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventID(pub Uuid);

impl EventID {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EventID {
    fn default() -> Self {
        Self::new()
    }
}
