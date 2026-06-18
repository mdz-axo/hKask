//! User sovereignty and affirmative consent types
//!
//! These types enforce the Magna Carta of hKask:
//! - Clear boundaries that honor user sovereignty
//! - Affirmative consent (default deny, explicit yes required)
//! - Data sovereignty boundaries (sovereign/shared/public)

use crate::id::SovereigntyId;
use crate::visibility::Visibility;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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
    /// REQ: TYP-143
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
    /// REQ: TYP-144
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
    /// REQ: TYP-145
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
    /// level applies to each data category. It encodes the 6-loop model's
    /// public/private/shared distinction:
    /// - Private: episodic memory, personal context, capability tokens, OCAP boundaries
    /// - Shared: semantic memory, template invocations
    /// - Public: hLexicon terms, template registry
    ///
    /// Get default visibility for this category.
    ///
    /// REQ: TYP-146
    /// post: returns Private for sovereign, Public for shared categories
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
///
/// Single source of truth for the SOVEREIGN/SHARED/PUBLIC/UNKNOWN mapping
/// previously duplicated across CLI, API, and verification service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryClassification {
    Sovereign,
    Shared,
    Public,
    Unknown,
}

impl BoundaryClassification {
    /// Human-readable label.
    /// Get human-readable label.
    ///
    /// REQ: TYP-147
    /// post: returns label string
    pub fn label(&self) -> &'static str {
        match self {
            BoundaryClassification::Sovereign => "SOVEREIGN",
            BoundaryClassification::Shared => "SHARED",
            BoundaryClassification::Public => "PUBLIC",
            BoundaryClassification::Unknown => "UNKNOWN",
        }
    }

    /// Access requirement description.
    /// Get access level required.
    ///
    /// REQ: TYP-148
    /// post: returns "sovereign", "shared", or "public"
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

    /// Create boundary with typical hKask defaults.
    ///
    /// This is the canonical boundary classification referenced by the
    /// Magna Carta (Data Sovereignty Boundary section). Surfaced as a
    /// public constructor so external crates (CLI, API) can render the
    /// same default that runtime types use.
    /// Create hKask default sovereignty state.
    ///
    /// REQ: TYP-149
    /// post: returns UserSovereigntyState with all categories sovereign
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

    /// Check if data category is under user sovereignty
    /// Check if a category is sovereign.
    ///
    /// REQ: TYP-150
    /// pre:  category is valid
    /// post: returns true iff category is in sovereign set
    pub fn is_sovereign(&self, category: &DataCategory) -> bool {
        self.sovereign_data.contains(category)
    }

    /// Check if data category is in shared set
    ///
    /// F-SYN-003: renamed from `is_shared` to `is_category_shared` to
    /// resolve the name collision with the (now-removed)
    /// `Visibility::is_shared` predicate. The new name is
    /// self-documenting about what the predicate operates on.
    /// Check if a category is shared.
    ///
    /// REQ: TYP-151
    /// pre:  category is valid
    /// post: returns true iff category is in shared set
    pub fn is_category_shared(&self, category: &DataCategory) -> bool {
        self.shared_data.contains(category)
    }

    /// Check if data category is public
    ///
    /// F-SYN-003: same rationale as `is_category_shared`.
    /// Check if a category is public.
    ///
    /// REQ: TYP-152
    /// pre:  category is valid
    /// post: returns true iff category is in public set
    pub fn is_category_public(&self, category: &DataCategory) -> bool {
        self.public_data.contains(category)
    }

    /// Whether this boundary requires affirmative consent (default: true)
    /// Check if affirmative consent is required.
    ///
    /// REQ: TYP-153
    /// post: returns true (always required under Magna Carta)
    pub fn requires_affirmative_consent(&self) -> bool {
        self.requires_affirmative_consent
    }

    /// Classify a data category within this boundary.
    ///
    /// Single source of truth for the SOVEREIGN/SHARED/PUBLIC/UNKNOWN mapping
    /// previously duplicated across CLI, API, and verification service.
    /// Classify a category's boundary.
    ///
    /// REQ: TYP-154
    /// pre:  category is valid
    /// post: returns BoundaryClassification (Sovereign, Shared, or Public)
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
    /// Whether user has explicitly consented to data sharing
    pub explicit_consent: bool,
    /// Timestamp of last sovereignty check
    pub last_check: chrono::DateTime<chrono::Utc>,
}

impl UserSovereigntyState {
    /// Create a new consent state.
    ///
    /// REQ: TYP-155
    /// post: returns ConsentState with consent=false
    pub fn new() -> Self {
        Self {
            boundary: DataSovereigntyBoundary::hkask_default(),
            explicit_consent: false,
            last_check: chrono::Utc::now(),
        }
    }

    /// Grant explicit consent for data sharing
    /// Grant consent.
    ///
    /// REQ: TYP-156
    /// post: consent set to true
    pub fn grant_consent(&mut self) {
        self.explicit_consent = true;
    }

    /// Revoke explicit consent
    /// Revoke consent.
    ///
    /// REQ: TYP-157
    /// post: consent set to false
    pub fn revoke_consent(&mut self) {
        self.explicit_consent = false;
    }
}

impl Default for UserSovereigntyState {
    fn default() -> Self {
        Self::new()
    }
}
