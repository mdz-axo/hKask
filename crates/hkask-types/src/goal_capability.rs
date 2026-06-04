//! Goal capability types — Cross-cutting infrastructure
//!
//! Capability tokens for goal operations. Governed by Cybernetics (6.1 Access Guard)
//! but the goal domain itself spans multiple loops.

use crate::capability::SYSTEM_MAX_ATTENUATION;
use crate::capability::hmac_ops;
use crate::id::{GoalID, WebID};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Goal operations — what capabilities can authorize
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GoalOp {
    Create,
    Read,
    Update,
    Verify,
    Complete,
    CreateSubgoal,
    AddArtifact,
}

impl GoalOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            GoalOp::Create => "CREATE",
            GoalOp::Read => "READ",
            GoalOp::Update => "UPDATE",
            GoalOp::Verify => "VERIFY",
            GoalOp::Complete => "COMPLETE",
            GoalOp::CreateSubgoal => "CREATE_SUBGOAL",
            GoalOp::AddArtifact => "ADD_ARTIFACT",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "CREATE" => Some(GoalOp::Create),
            "READ" => Some(GoalOp::Read),
            "UPDATE" => Some(GoalOp::Update),
            "VERIFY" => Some(GoalOp::Verify),
            "COMPLETE" => Some(GoalOp::Complete),
            "CREATE_SUBGOAL" => Some(GoalOp::CreateSubgoal),
            "ADD_ARTIFACT" => Some(GoalOp::AddArtifact),
            _ => None,
        }
    }
}

/// Goal capability token — OCAP authorization.
///
/// The HMAC signature binds **every authority-bearing field**: id, goal_id,
/// holder, the (canonicalized) operation set, expiry, both attenuation
/// fields, and the epoch counter. A holder therefore cannot append operations,
/// extend expiry, raise the attenuation ceiling, or reuse a token after
/// epoch-based revocation without invalidating the signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalCapabilityToken {
    pub id: String,
    pub goal_id: GoalID,
    pub holder_webid: WebID,
    pub operations: Vec<GoalOp>,
    pub expires: DateTime<Utc>,
    pub attenuation_level: u8,
    /// Maximum attenuation level allowed (prevents unbounded delegation).
    /// Capped at [`SYSTEM_MAX_ATTENUATION`] to align with ADR-025 and the
    /// canonical `CapabilityToken`.
    pub max_attenuation: u8,
    pub hmac_signature: String,
    /// Epoch counter for revocation. When the stored epoch advances (e.g. on
    /// revocation), all tokens carrying the old epoch become invalid.
    pub epoch: u64,
}

impl GoalCapabilityToken {
    pub fn new(
        goal_id: GoalID,
        holder_webid: WebID,
        operations: Vec<GoalOp>,
        secret: &[u8],
    ) -> Self {
        let id = format!("gct_{}", uuid::Uuid::new_v4().simple());
        let expires = Utc::now() + chrono::Duration::hours(24);

        let mut token = Self {
            id,
            goal_id,
            holder_webid,
            operations,
            expires,
            attenuation_level: 0,
            max_attenuation: SYSTEM_MAX_ATTENUATION,
            hmac_signature: String::new(),
            epoch: 0,
        };

        token.hmac_signature = token.compute_hmac(secret);
        token
    }

    /// Canonical, order-independent encoding of the operation set.
    ///
    /// Operations are deduplicated and sorted by their stable string form so
    /// the signature is invariant to ordering and duplicate entries — a
    /// reordered `operations` vector cannot change the authorized set without
    /// breaking the HMAC.
    fn canonical_operations(&self) -> Vec<&'static str> {
        let mut ops: Vec<&'static str> = self.operations.iter().map(GoalOp::as_str).collect();
        ops.sort_unstable();
        ops.dedup();
        ops
    }

    fn compute_hmac(&self, secret: &[u8]) -> String {
        let mut builder = hmac_ops::HmacBuilder::new(secret);
        builder.update(self.id.as_bytes());
        builder.update(self.goal_id.to_string().as_bytes());
        builder.update(self.holder_webid.to_string().as_bytes());
        builder.update(self.attenuation_level.to_string().as_bytes());
        builder.update(self.max_attenuation.to_string().as_bytes());
        builder.update(self.epoch.to_be_bytes().as_slice());
        builder.update(self.expires.to_rfc3339().as_bytes());
        // Bind the operation set. A length-delimited encoding prevents
        // adjacent operations from being merged or split ambiguously.
        for op in self.canonical_operations() {
            builder.update((op.len() as u32).to_be_bytes().as_slice());
            builder.update(op.as_bytes());
        }
        builder.finalize_hex()
    }

    /// Verify the signature using constant-time comparison to avoid leaking
    /// the expected HMAC byte-by-byte through timing.
    pub fn verify_signature(&self, secret: &[u8]) -> bool {
        let expected = self.compute_hmac(secret);
        hmac_ops::verify_hmac_constant_time(expected.as_bytes(), self.hmac_signature.as_bytes())
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires
    }

    pub fn is_valid(&self, secret: &[u8], current_epoch: u64) -> bool {
        !self.is_expired() && self.epoch == current_epoch && self.verify_signature(secret)
    }

    pub fn can_perform(&self, operation: GoalOp, secret: &[u8], current_epoch: u64) -> bool {
        self.is_valid(secret, current_epoch) && self.operations.contains(&operation)
    }

    /// Whether this token may be further attenuated.
    ///
    /// Mirrors `CapabilityToken::can_attenuate()` so both token families share
    /// one delegation-depth semantics (ADR-025).
    pub fn can_attenuate(&self) -> bool {
        self.attenuation_level < self.max_attenuation
    }

    pub fn attenuate(&self, new_operations: Vec<GoalOp>, secret: &[u8]) -> Option<Self> {
        if !self.can_attenuate() {
            return None;
        }

        let mut attenuated =
            GoalCapabilityToken::new(self.goal_id, self.holder_webid, new_operations, secret);
        attenuated.attenuation_level = self.attenuation_level + 1;
        attenuated.max_attenuation = self.max_attenuation;
        attenuated.epoch = self.epoch;
        attenuated.expires = Utc::now() + (self.expires - Utc::now()) / 2;
        attenuated.hmac_signature = attenuated.compute_hmac(secret);
        Some(attenuated)
    }
}
