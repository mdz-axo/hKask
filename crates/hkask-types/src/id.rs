//! ID types for hKask entities

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// WebID — Unique identifier for agents (bots and replicants)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl std::fmt::Display for WebID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// TemplateID — Unique identifier for templates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl std::fmt::Display for TemplateID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// BotID — Unique identifier for bots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl std::fmt::Display for BotID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// ManifestID — Unique identifier for manifests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl std::fmt::Display for ManifestID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// TripleID — Unique identifier for bitemporal triples
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl std::fmt::Display for TripleID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// EventID — Unique identifier for ν-events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl std::fmt::Display for EventID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}


