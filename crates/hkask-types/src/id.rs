//! ID types for hKask entities

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Macro to define UUID-based ID types with common functionality
///
/// # Example
/// ```ignore
/// define_id_type!(BotID);
/// define_id_type!(TemplateID, from_string);
/// ```
#[macro_export]
macro_rules! define_id_type {
    // Basic ID type with just new()
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub ::uuid::Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(::uuid::Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };

    // ID type with from_string() method
    ($name:ident, from_string) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub ::uuid::Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(::uuid::Uuid::new_v4())
            }

            pub fn from_string(s: &str) -> Self {
                Self(::uuid::Uuid::parse_str(s).unwrap_or_else(|_| ::uuid::Uuid::new_v4()))
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

pub use crate::define_id_type;

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
    ///
    /// Note: This uses a default namespace. For namespace isolation,
    /// use `from_persona_with_namespace` instead.
    pub fn from_persona(persona_bytes: &[u8]) -> Self {
        Self::from_persona_with_namespace(persona_bytes, "hkask")
    }

    /// Derive WebID deterministically from persona with namespace isolation (R10)
    ///
    /// Uses SHA-1 name-based UUID with a fixed namespace.
    /// Combines namespace and persona bytes to prevent collisions across
    /// different agent registries (e.g., "hkask" vs "russell").
    ///
    /// Same namespace + persona bytes → same WebID.
    pub fn from_persona_with_namespace(persona_bytes: &[u8], namespace: &str) -> Self {
        // Fixed namespace UUID for hKask personas
        // UUID: 686b6173-6b2d-7065-7273-6f6e612d6e73
        let base_namespace = Uuid::parse_str("686b6173-6b2d-7065-7273-6f6e612d6e73")
            .expect("Invalid namespace UUID");

        // Combine namespace and persona bytes to create isolated WebIDs
        let mut combined = Vec::with_capacity(namespace.len() + 1 + persona_bytes.len());
        combined.extend_from_slice(namespace.as_bytes());
        combined.push(b':');
        combined.extend_from_slice(persona_bytes);

        Self(Uuid::new_v5(&base_namespace, &combined))
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
define_id_type!(TemplateID, from_string);

/// BotID — Unique identifier for bots
define_id_type!(BotID);

impl From<BotID> for WebID {
    fn from(bot_id: BotID) -> Self {
        WebID(bot_id.0)
    }
}

/// ManifestID — Unique identifier for manifests
define_id_type!(ManifestID);

/// TripleID — Unique identifier for bitemporal triples
define_id_type!(TripleID);

/// EventID — Unique identifier for ν-events
define_id_type!(EventID);

/// SessionID — Unique identifier for agent sessions
define_id_type!(SessionID);

/// GoalID — Unique identifier for goals
define_id_type!(GoalID, from_string);

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

    #[test]
    fn test_webid_namespace_isolation() {
        let persona = b"Curator";
        let id_hkask = WebID::from_persona_with_namespace(persona, "hkask");
        let id_russell = WebID::from_persona_with_namespace(persona, "russell");
        let id_default = WebID::from_persona(persona);

        // Different namespaces should produce different WebIDs
        assert_ne!(
            id_hkask, id_russell,
            "Different namespaces should produce different WebIDs for same persona"
        );

        // Default namespace should match "hkask" namespace
        assert_eq!(
            id_hkask, id_default,
            "Default namespace should match explicit 'hkask' namespace"
        );
    }

    #[test]
    fn test_webid_namespace_deterministic() {
        let persona = b"test-agent";
        let namespace = "test-namespace";
        let id1 = WebID::from_persona_with_namespace(persona, namespace);
        let id2 = WebID::from_persona_with_namespace(persona, namespace);
        assert_eq!(
            id1, id2,
            "Same namespace + persona should produce same WebID"
        );
    }
}
