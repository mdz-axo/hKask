//! ID types for hKask entities

// G2 Justification: This module exposes 25 public items because it defines strongly-typed ID newtypes for domain-driven design. Each ID type prevents accidental confusion between different entity kinds (WebID vs PodID vs GoalID). Merging would defeat their purpose.

use std::fmt::Debug;
use std::marker::PhantomData;

mod private {
    pub trait Sealed {}
}

/// Marker trait for ID kind — enables type-safe ID types via phantom generics.
/// The `Sealed` supertrait prevents external implementations.
pub trait IdKind: private::Sealed + 'static {}

/// Generic UUID-based identifier with phantom type parameter.
///
/// `Id<BotKind>` and `Id<TemplateKind>` are different types — you can't
/// accidentally pass a `BotID` where a `TemplateID` is expected. All common
/// functionality (construction, parsing, display, hashing) lives here once.
pub struct Id<T: IdKind> {
    uuid: Uuid,
    _marker: PhantomData<T>,
}

// ── Manual trait impls (avoid derived bounds on phantom type parameter T) ──

impl<T: IdKind> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: IdKind> Copy for Id<T> {}

impl<T: IdKind> Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Id").field(&self.uuid).finish()
    }
}

impl<T: IdKind> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl<T: IdKind> Eq for Id<T> {}

impl<T: IdKind> Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}

impl<T: IdKind> serde::Serialize for Id<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.uuid.serialize(serializer)
    }
}

impl<'de, T: IdKind> serde::Deserialize<'de> for Id<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Uuid::deserialize(deserializer).map(Id::from_uuid)
    }
}

impl<T: IdKind> Id<T> {
    pub fn new() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            _marker: PhantomData,
        }
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self {
            uuid,
            _marker: PhantomData,
        }
    }

    /// Derive a deterministic Id from a name string using UUID v5.
    ///
    /// Same name → same Id. Useful for creating stable identifiers
    /// for entities that need to be looked up by name (e.g., wallets
    /// bound to replicant names).
    pub fn from_name(name: &str) -> Self {
        let namespace = Uuid::parse_str("686b6173-6b2d-7065-7273-6f6e612d6e73")
            .expect("Invalid namespace UUID");
        Self::from_uuid(Uuid::new_v5(&namespace, name.as_bytes()))
    }

    pub fn as_uuid(&self) -> Uuid {
        self.uuid
    }
}

impl<T: IdKind> std::str::FromStr for Id<T> {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self::from_uuid)
    }
}

impl<T: IdKind> Default for Id<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: IdKind> std::fmt::Display for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.uuid)
    }
}

pub enum TemplateKind {}
impl private::Sealed for TemplateKind {}
impl IdKind for TemplateKind {}

pub enum BotKind {}
impl private::Sealed for BotKind {}
impl IdKind for BotKind {}

pub enum TripleKind {}
impl private::Sealed for TripleKind {}
impl IdKind for TripleKind {}

pub enum EventKind {}
impl private::Sealed for EventKind {}
impl IdKind for EventKind {}

pub enum GoalKind {}
impl private::Sealed for GoalKind {}
impl IdKind for GoalKind {}

pub enum EmbeddingKind {}
impl private::Sealed for EmbeddingKind {}
impl IdKind for EmbeddingKind {}

pub enum UserKind {}
impl private::Sealed for UserKind {}
impl IdKind for UserKind {}

pub(crate) enum SovereigntyKind {}
impl private::Sealed for SovereigntyKind {}
impl IdKind for SovereigntyKind {}

pub enum PodKind {}
impl private::Sealed for PodKind {}
impl IdKind for PodKind {}

pub enum WalletKind {}
impl private::Sealed for WalletKind {}
impl IdKind for WalletKind {}

pub enum ApiKeyKind {}
impl private::Sealed for ApiKeyKind {}
impl IdKind for ApiKeyKind {}

pub enum EscalationKind {}
impl private::Sealed for EscalationKind {}
impl IdKind for EscalationKind {}

pub type TemplateID = Id<TemplateKind>;
pub type BotID = Id<BotKind>;
pub type TripleID = Id<TripleKind>;
pub type EventID = Id<EventKind>;
pub type GoalID = Id<GoalKind>;
pub type EmbeddingID = Id<EmbeddingKind>;
pub type UserID = Id<UserKind>;
pub(crate) type SovereigntyId = Id<SovereigntyKind>;
pub type PodID = Id<PodKind>;
pub type WalletId = Id<WalletKind>;
pub type ApiKeyId = Id<ApiKeyKind>;
pub type EscalationID = Id<EscalationKind>;

use std::hash::Hash;
use uuid::Uuid;

/// WebID — Unique identifier for agents (bots and replicants)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct WebID(Uuid);

impl WebID {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
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
    /// different agent registries.
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

    /// Redacted display format — shows first 8 chars of UUID + "..."
    /// Use at INFO level and below to prevent full UUID leakage in logs.
    pub fn redacted_display(&self) -> String {
        let full = self.0.to_string();
        format!("{}...", &full[..8])
    }

    /// Full display format — shows complete UUID.
    /// Use only at TRACE level with HKASK_TRACE_WEBIDS=1.
    #[allow(dead_code)] // reserved for future trace-level diagnostics
    pub(crate) fn full_display(&self) -> String {
        self.0.to_string()
    }
}

impl std::str::FromStr for WebID {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(WebID)
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

impl From<BotID> for WebID {
    fn from(bot_id: BotID) -> Self {
        WebID(bot_id.as_uuid())
    }
}
