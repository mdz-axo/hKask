//! Core ID system — generic UUID-based identifiers with phantom type parameters.
//!
//! G2 Justification: 7 public items (IdKind, Id, 5 kind types). The generic `Id<T>`
//! struct and its kind types are a single cohesive abstraction — splitting them
//! would break the phantom-type pattern that prevents accidental ID confusion.

use std::fmt::Debug;
use std::marker::PhantomData;
use uuid::Uuid;

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

impl<T: IdKind> std::hash::Hash for Id<T> {
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
    /// REQ: TYP-257
    /// pre:  (no inputs)
    /// post: returns a unique [`Id<T>`] wrapping a random UUID v4
    pub fn new() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            _marker: PhantomData,
        }
    }

    /// REQ: TYP-258
    /// pre:  uuid is any valid [`Uuid`]
    /// post: returns an [`Id<T>`] wrapping the given uuid unchanged
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
    ///
    /// REQ: TYP-259
    /// pre:  name is any non-empty string (empty produces a deterministic but degenerate Id)
    /// post: returns an [`Id<T>`] deterministically derived from name using UUID v5;
    ///       same name → same Id
    pub fn from_name(name: &str) -> Self {
        let namespace = Uuid::parse_str("686b6173-6b2d-7065-7273-6f6e612d6e73")
            .expect("Invalid namespace UUID");
        Self::from_uuid(Uuid::new_v5(&namespace, name.as_bytes()))
    }

    /// REQ: TYP-260
    /// pre:  self is any valid [`Id<T>`]
    /// post: returns the inner [`Uuid`] unchanged
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

// ── Kind types (sealed, empty enums for phantom type parameters) ─────────

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

// ── Type aliases ──────────────────────────────────────────────────────────

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
