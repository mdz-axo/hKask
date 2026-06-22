//! DataCategory — sovereignty classification for data access control
//!
//! These types enforce the Magna Carta of hKask:
//! - Clear boundaries that honor user sovereignty
//! - Affirmative consent (default deny, explicit yes required)
//! - Data sovereignty boundaries (sovereign/shared/public)

use crate::visibility::Visibility;
use serde::{Deserialize, Serialize};

/// Data category for sovereignty classification
///
/// Categories determine what sovereignty rules apply:
/// - Sovereign: Requires explicit user consent and ownership
/// - Shared: Requires explicit consent
/// - Public: Always accessible
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataCategory {
    /// Episodic memory (private, personal experiences)
    EpisodicMemory,
    /// Semantic memory (shared knowledge, facts)
    SemanticMemory,
    /// Personal context (user-specific settings, preferences)
    PersonalContext,
    /// Capability tokens (OCAP credentials)
    CapabilityTokens,
    /// OCAP boundaries (access control rules)
    OcapBoundaries,
    /// Template invocations (prompt/render history)
    TemplateInvocations,
    /// Template registry (public template metadata)
    TemplateRegistry,
    /// Custom category (application-specific)
    Custom(String),
}

impl DataCategory {
    /// Get string representation of data category
    /// Get string representation of category.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns category name as &str
    pub fn as_str(&self) -> &str {
        match self {
            DataCategory::EpisodicMemory => "episodic_memory",
            DataCategory::SemanticMemory => "semantic_memory",
            DataCategory::PersonalContext => "personal_context",
            DataCategory::CapabilityTokens => "capability_tokens",
            DataCategory::OcapBoundaries => "ocap_boundaries",
            DataCategory::TemplateInvocations => "template_invocations",
            DataCategory::TemplateRegistry => "template_registry",
            DataCategory::Custom(s) => s.as_str(),
        }
    }

    /// Parse a data category from its string representation.
    ///
    /// Known categories map directly; unknown strings become `DataCategory::Custom`.
    /// This is the single source of truth — replaces the 3 duplicated `parse_data_category`
    /// functions previously scattered across CLI helpers, CLI sovereignty, and API routes.
    /// Parse a DataCategory from string.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns DataCategory (defaults to Episodic for unknown)
    pub fn parse(s: &str) -> Self {
        match s {
            "episodic_memory" => DataCategory::EpisodicMemory,
            "semantic_memory" => DataCategory::SemanticMemory,
            "personal_context" => DataCategory::PersonalContext,
            "capability_tokens" => DataCategory::CapabilityTokens,
            "ocap_boundaries" => DataCategory::OcapBoundaries,
            "template_invocations" => DataCategory::TemplateInvocations,
            "template_registry" => DataCategory::TemplateRegistry,
            _ => DataCategory::Custom(s.to_string()),
        }
    }

    /// Check if this category is typically sovereign
    /// Check if this category is typically sovereign.
    ///
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// post: returns true for Episodic, Goals, Wallet, Identity
    pub fn is_typically_sovereign(&self) -> bool {
        matches!(
            self,
            DataCategory::EpisodicMemory
                | DataCategory::PersonalContext
                | DataCategory::CapabilityTokens
                | DataCategory::OcapBoundaries
        )
    }
}

impl std::fmt::Display for DataCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl DataCategory {
    /// Canonical visibility for this data category.
    ///
    /// This mapping is the single source of truth for which visibility
    /// level applies to each data category.
    pub fn default_visibility(&self) -> Visibility {
        match self {
            Self::EpisodicMemory
            | Self::PersonalContext
            | Self::CapabilityTokens
            | Self::OcapBoundaries => Visibility::Private,
            Self::SemanticMemory | Self::TemplateInvocations => Visibility::Public,
            Self::TemplateRegistry => Visibility::Public,
            Self::Custom(_) => Visibility::Private, // conservative default
        }
    }
}

// ── Sovereignty boundary types ───────────────────────────────────────────

use crate::SovereigntyId;
use std::collections::HashSet;

/// Data sovereignty boundary — defines what data the user controls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSovereigntyBoundary {
    pub(crate) id: SovereigntyId,
    /// What data is under user sovereignty
    pub sovereign_data: HashSet<DataCategory>,
    /// What data may be shared (with explicit consent)
    pub shared_data: HashSet<DataCategory>,
    /// What data is public (no sovereignty claim)
    pub public_data: HashSet<DataCategory>,
    /// Whether this boundary requires affirmative consent (default: true)
    pub(crate) requires_affirmative_consent: bool,
}

/// Classification of a data category within a sovereignty boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryClassification {
    Sovereign,
    Shared,
    Public,
    Unknown,
}

impl BoundaryClassification {
    pub fn label(&self) -> &'static str {
        match self {
            BoundaryClassification::Sovereign => "SOVEREIGN",
            BoundaryClassification::Shared => "SHARED",
            BoundaryClassification::Public => "PUBLIC",
            BoundaryClassification::Unknown => "UNKNOWN",
        }
    }

    pub fn access_required(&self) -> &'static str {
        match self {
            BoundaryClassification::Sovereign => "Requires explicit consent AND owner",
            BoundaryClassification::Shared => "Requires explicit consent",
            BoundaryClassification::Public => "Always accessible",
            BoundaryClassification::Unknown => "Denied by default",
        }
    }
}

impl DataSovereigntyBoundary {
    pub(crate) fn new() -> Self {
        Self {
            id: SovereigntyId::default(),
            sovereign_data: HashSet::new(),
            shared_data: HashSet::new(),
            public_data: HashSet::new(),
            requires_affirmative_consent: false,
        }
    }

    pub fn hkask_default() -> Self {
        let mut sovereign_data = HashSet::new();
        sovereign_data.insert(DataCategory::EpisodicMemory);
        sovereign_data.insert(DataCategory::PersonalContext);
        sovereign_data.insert(DataCategory::CapabilityTokens);
        sovereign_data.insert(DataCategory::OcapBoundaries);

        let mut shared_data = HashSet::new();
        shared_data.insert(DataCategory::SemanticMemory);
        shared_data.insert(DataCategory::TemplateInvocations);

        let mut public_data = HashSet::new();
        public_data.insert(DataCategory::TemplateRegistry);

        Self {
            id: SovereigntyId::default(),
            sovereign_data,
            shared_data,
            public_data,
            requires_affirmative_consent: true,
        }
    }

    pub fn is_sovereign(&self, category: &DataCategory) -> bool {
        self.sovereign_data.contains(category)
    }

    pub fn is_category_shared(&self, category: &DataCategory) -> bool {
        self.shared_data.contains(category)
    }

    pub fn is_category_public(&self, category: &DataCategory) -> bool {
        self.public_data.contains(category)
    }

    pub fn requires_affirmative_consent(&self) -> bool {
        self.requires_affirmative_consent
    }

    pub fn classify(&self, category: &DataCategory) -> BoundaryClassification {
        if self.is_sovereign(category) {
            BoundaryClassification::Sovereign
        } else if self.is_category_shared(category) {
            BoundaryClassification::Shared
        } else if self.is_category_public(category) {
            BoundaryClassification::Public
        } else {
            BoundaryClassification::Unknown
        }
    }
}

impl Default for DataSovereigntyBoundary {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSovereigntyState {
    pub boundary: DataSovereigntyBoundary,
    pub explicit_consent: bool,
    pub last_check: chrono::DateTime<chrono::Utc>,
}

impl UserSovereigntyState {
    pub fn new() -> Self {
        Self {
            boundary: DataSovereigntyBoundary::hkask_default(),
            explicit_consent: false,
            last_check: chrono::Utc::now(),
        }
    }

    pub fn grant_consent(&mut self) {
        self.explicit_consent = true;
    }

    pub fn revoke_consent(&mut self) {
        self.explicit_consent = false;
    }
}

impl Default for UserSovereigntyState {
    fn default() -> Self {
        Self::new()
    }
}

// ── Original curation types restored ────────────────────────────────────

/// CurationDecision — The Curator's evaluation of template outputs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurationDecision {
    /// Merge output into codebase
    Merge,
    /// Discard output entirely
    Discard,
    /// Request revision from bot
    Revise,
    /// Insufficient information — revisit later
    Defer,
}

impl std::fmt::Display for CurationDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CurationDecision::Merge => write!(f, "merge"),
            CurationDecision::Discard => write!(f, "discard"),
            CurationDecision::Revise => write!(f, "revise"),
            CurationDecision::Defer => write!(f, "defer"),
        }
    }
}

impl TryFrom<&str> for CurationDecision {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "merge" => Ok(CurationDecision::Merge),
            "discard" => Ok(CurationDecision::Discard),
            "revise" => Ok(CurationDecision::Revise),
            "defer" => Ok(CurationDecision::Defer),
            _ => Err(format!("invalid curation decision: {s}")),
        }
    }
}

/// Token-based capability kinds for OCAP boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OcapTokenKind {
    /// Curation authority — ConsolidationToken
    Curation,
    /// Spec curation authority
    SpecCurate,
    /// Federation authority — manage federation links, invitations, sync
    Federation,
}

impl std::fmt::Display for OcapTokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            OcapTokenKind::Curation => "curation",
            OcapTokenKind::SpecCurate => "spec_curate",
            OcapTokenKind::Federation => "federation",
        };
        f.write_str(s)
    }
}

/// Parse an `OcapTokenKind` from its canonical snake_case name.
pub fn parse_ocap_token_kind(s: &str) -> Option<OcapTokenKind> {
    match s {
        "curation" => Some(OcapTokenKind::Curation),
        "spec_curate" => Some(OcapTokenKind::SpecCurate),
        "federation" => Some(OcapTokenKind::Federation),
        _ => None,
    }
}

/// Capability identifier — typed brand.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OcapCapability(pub OcapTokenKind);

impl std::fmt::Display for OcapCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

/// OCAPBoundary — Capability boundary for curation decisions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OCAPBoundary {
    /// The capability being bounded (a typed brand).
    pub capability: OcapCapability,
}

impl OCAPBoundary {
    /// Create an enforced boundary with a typed token.
    pub fn token(kind: OcapTokenKind) -> Self {
        Self {
            capability: OcapCapability(kind),
        }
    }

    /// Parse a typed token from a string, returning `None` for unknown names.
    pub fn parse_token(name: &str) -> Option<Self> {
        parse_ocap_token_kind(name).map(Self::token)
    }
}
