//! User sovereignty and anti-catch-kill types
//!
//! These types enforce the Magna Carta of hKask:
//! - Clear boundaries that honor user sovereignty
//! - Acquisition resistance mechanisms
//! - Kill-zone detection for VC investment patterns

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
    /// hLexicon terms (canonical vocabulary)
    HLexiconTerms,
    /// Template registry (public template metadata)
    TemplateRegistry,
    /// Custom category (application-specific)
    Custom(String),
}

impl DataCategory {
    /// Get string representation of data category
    pub fn as_str(&self) -> &str {
        match self {
            DataCategory::EpisodicMemory => "episodic_memory",
            DataCategory::SemanticMemory => "semantic_memory",
            DataCategory::PersonalContext => "personal_context",
            DataCategory::CapabilityTokens => "capability_tokens",
            DataCategory::OcapBoundaries => "ocap_boundaries",
            DataCategory::TemplateInvocations => "template_invocations",
            DataCategory::HLexiconTerms => "hlexicon_terms",
            DataCategory::TemplateRegistry => "template_registry",
            DataCategory::Custom(s) => s.as_str(),
        }
    }

    /// Check if this category is typically sovereign
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
    pub fn default_visibility(&self) -> Visibility {
        match self {
            Self::EpisodicMemory
            | Self::PersonalContext
            | Self::CapabilityTokens
            | Self::OcapBoundaries => Visibility::Private,
            Self::SemanticMemory | Self::TemplateInvocations => Visibility::Shared,
            Self::HLexiconTerms | Self::TemplateRegistry => Visibility::Public,
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
    /// Whether this boundary resists passive acquisition
    pub(crate) acquisition_resistance: bool,
}

impl DataSovereigntyBoundary {
    pub(crate) fn new() -> Self {
        Self {
            id: SovereigntyId::default(),
            sovereign_data: HashSet::new(),
            shared_data: HashSet::new(),
            public_data: HashSet::new(),
            acquisition_resistance: false,
        }
    }

    /// Create boundary with typical hKask defaults
    pub(crate) fn hkask_default() -> Self {
        let mut sovereign_data = HashSet::new();
        sovereign_data.insert(DataCategory::EpisodicMemory);
        sovereign_data.insert(DataCategory::PersonalContext);
        sovereign_data.insert(DataCategory::CapabilityTokens);
        sovereign_data.insert(DataCategory::OcapBoundaries);

        let mut shared_data = HashSet::new();
        shared_data.insert(DataCategory::SemanticMemory);
        shared_data.insert(DataCategory::TemplateInvocations);

        let mut public_data = HashSet::new();
        public_data.insert(DataCategory::HLexiconTerms);
        public_data.insert(DataCategory::TemplateRegistry);

        Self {
            id: SovereigntyId::default(),
            sovereign_data,
            shared_data,
            public_data,
            acquisition_resistance: true,
        }
    }

    /// Check if data category is under user sovereignty
    pub fn is_sovereign(&self, category: &DataCategory) -> bool {
        self.sovereign_data.contains(category)
    }

    /// Check if data category is in shared set
    pub fn is_shared(&self, category: &DataCategory) -> bool {
        self.shared_data.contains(category)
    }

    /// Check if data category is public
    pub fn is_public(&self, category: &DataCategory) -> bool {
        self.public_data.contains(category)
    }

    /// Whether this boundary resists passive acquisition
    pub fn prevents_passive_acquisition(&self) -> bool {
        self.acquisition_resistance
    }
}

impl Default for DataSovereigntyBoundary {
    fn default() -> Self {
        Self::new()
    }
}

/// Kill zone state — mutable operational state for kill-zone detection.
///
/// The detection logic lives in hkask-cns (Cybernetics subloop 6.5).
/// This struct holds the operational state that CNS senses and compares.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillZoneState {
    /// Current VC investment level (0.0 to 1.0)
    pub vc_investment: f32,
    /// Whether kill zone is currently detected
    pub kill_zone_active: bool,
    /// Whether an acquisition attempt has been detected
    pub acquisition_attempt: bool,
}

impl Default for KillZoneState {
    fn default() -> Self {
        Self {
            vc_investment: 1.0,
            kill_zone_active: false,
            acquisition_attempt: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSovereigntyState {
    pub boundary: DataSovereigntyBoundary,
    pub kill_zone_state: KillZoneState,
    /// Kill zone thresholds (set by Curation, immutable at runtime)
    #[serde(skip)]
    pub kill_zone_threshold: f32,
    /// Whether user has explicitly consented to data sharing
    pub explicit_consent: bool,
    /// Timestamp of last sovereignty check
    pub last_check: chrono::DateTime<chrono::Utc>,
}

impl UserSovereigntyState {
    pub fn new() -> Self {
        Self {
            boundary: DataSovereigntyBoundary::hkask_default(),
            kill_zone_state: KillZoneState::default(),
            kill_zone_threshold: 0.5,
            explicit_consent: false,
            last_check: chrono::Utc::now(),
        }
    }

    /// Update sovereignty state with current VC investment
    pub fn update_vc_investment(&mut self, vc_investment: f32) {
        self.kill_zone_state.vc_investment = vc_investment.clamp(0.0, 1.0);
        self.kill_zone_state.kill_zone_active = self.kill_zone_state.acquisition_attempt
            && self.kill_zone_state.vc_investment < self.kill_zone_threshold;
        self.last_check = chrono::Utc::now();
    }

    /// Mark acquisition attempt
    pub fn mark_acquisition_attempt(&mut self) {
        self.kill_zone_state.acquisition_attempt = true;
        self.kill_zone_state.kill_zone_active = self.kill_zone_state.acquisition_attempt
            && self.kill_zone_state.vc_investment < self.kill_zone_threshold;
        self.last_check = chrono::Utc::now();
    }

    /// Check if sovereignty is compromised
    pub fn is_compromised(&self) -> bool {
        self.kill_zone_state.kill_zone_active
    }

    /// Grant explicit consent for data sharing
    pub fn grant_consent(&mut self) {
        self.explicit_consent = true;
    }

    /// Revoke explicit consent
    pub fn revoke_consent(&mut self) {
        self.explicit_consent = false;
    }
}

impl Default for UserSovereigntyState {
    fn default() -> Self {
        Self::new()
    }
}

// Sovereignty Port Types

/// Sovereignty operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SovereigntyOperation {
    /// Data read operation
    Read,
    /// Data write operation
    Write,
    /// Data acquisition (passive collection)
    Acquisition,
    /// Data composition (combining multiple sources)
    Composition,
}

/// Sovereignty check result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SovereigntyCheckResult {
    /// Whether operation is allowed
    pub allowed: bool,
    /// Reason for denial (if any)
    pub denial_reason: Option<String>,
    /// Data category being accessed
    pub data_category: DataCategory,
    /// Operation type
    pub operation: SovereigntyOperation,
}

impl SovereigntyCheckResult {
    pub fn allowed(data_category: DataCategory, operation: SovereigntyOperation) -> Self {
        Self {
            allowed: true,
            denial_reason: None,
            data_category,
            operation,
        }
    }

    pub fn denied(
        data_category: DataCategory,
        operation: SovereigntyOperation,
        reason: &str,
    ) -> Self {
        Self {
            allowed: false,
            denial_reason: Some(reason.to_string()),
            data_category,
            operation,
        }
    }
}

/// Sovereignty port — abstraction for sovereignty checking
///
/// Implemented by `SovereigntyChecker` in `hkask-agents`.
/// Enables MCP and other infrastructure crates to depend on the
/// abstraction without depending on the orchestration crate.
pub trait SovereigntyPort: Send + Sync {
    /// Check if data category is accessible by requester
    fn can_access(&self, data_category: &DataCategory, requester: &crate::WebID) -> bool;
}
