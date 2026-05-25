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

    /// Derive WebID deterministically from persona using UUID v5
    ///
    /// Uses SHA-1 name-based UUID with a fixed namespace.
    /// Same persona bytes → same WebID.
    pub fn from_persona(persona_bytes: &[u8]) -> Self {
        // Fixed namespace UUID for hKask personas
        // UUID: 686b6173-6b2d-7065-7273-6f6e612d6e73
        let namespace = Uuid::parse_str("686b6173-6b2d-7065-7273-6f6e612d6e73")
            .expect("Invalid namespace UUID");

        Self(Uuid::new_v5(&namespace, persona_bytes))
    }

    pub fn from_string(s: &str) -> Self {
        WebID(uuid::Uuid::parse_str(s).unwrap_or_else(|_| uuid::Uuid::new_v4()))
    }

    /// Redacted display format — shows first 8 chars of UUID + "..."
    /// Use at INFO level and below to prevent full UUID leakage in logs.
    pub fn redacted_display(&self) -> String {
        let full = self.0.to_string();
        format!("{}...", &full[..8])
    }

    /// Full display format — shows complete UUID.
    /// Use only at TRACE level with HKASK_TRACE_WEBIDS=1.
    pub fn full_display(&self) -> String {
        self.0.to_string()
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

    pub fn from_string(s: &str) -> Self {
        TemplateID(Uuid::parse_str(s).unwrap_or_else(|_| Uuid::new_v4()))
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

impl From<BotID> for WebID {
    fn from(bot_id: BotID) -> Self {
        WebID(bot_id.0)
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

/// SessionID — Unique identifier for agent sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionID(pub Uuid);

impl SessionID {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SessionID {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// GoalID — Unique identifier for goals
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GoalID(pub Uuid);

impl GoalID {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Self {
        GoalID(uuid::Uuid::parse_str(s).unwrap_or_else(|_| uuid::Uuid::new_v4()))
    }
}

impl Default for GoalID {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for GoalID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webid_from_persona_deterministic() {
        let persona = b"test-persona-yaml";
        let id1 = WebID::from_persona(persona);
        let id2 = WebID::from_persona(persona);
        assert_eq!(id1, id2, "Same persona bytes should produce same WebID");
    }

    #[test]
    fn test_webid_from_persona_different() {
        let persona1 = b"persona-1";
        let persona2 = b"persona-2";
        let id1 = WebID::from_persona(persona1);
        let id2 = WebID::from_persona(persona2);
        assert_ne!(
            id1, id2,
            "Different persona bytes should produce different WebIDs"
        );
    }

    #[test]
    fn test_webid_from_persona_not_random() {
        let persona = b"fixed-persona";
        let id1 = WebID::from_persona(persona);
        let id2 = WebID::from_persona(persona);
        let id3 = WebID::from_persona(persona);
        assert_eq!(id1, id2);
        assert_eq!(id2, id3);
        assert_eq!(id1, id3);
    }
}
