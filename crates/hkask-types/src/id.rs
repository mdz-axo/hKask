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
///
/// Doc comments can be placed before the invocation — they will be attached
/// to the generated struct when using the `$(#[$meta:meta])*` capture.
#[macro_export]
macro_rules! define_id_type {
    // Basic ID type with visibility + optional attributes
    ($(#[$meta:meta])* $vis:vis $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        $vis struct $name(pub ::uuid::Uuid);

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

    // ID type with visibility, from_string() method + optional attributes
    ($(#[$meta:meta])* $vis:vis $name:ident, from_string) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        $vis struct $name(pub ::uuid::Uuid);

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

define_id_type!(pub TemplateID, from_string);

define_id_type!(pub BotID);

impl From<BotID> for WebID {
    fn from(bot_id: BotID) -> Self {
        WebID(bot_id.0)
    }
}

define_id_type!(pub(crate) ManifestID);

define_id_type!(pub TripleID);

define_id_type!(pub EventID);

define_id_type!(pub(crate) SessionID);

define_id_type!(pub GoalID, from_string);

define_id_type!(pub EmbeddingID);
