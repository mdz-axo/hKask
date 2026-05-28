//! Goal capability tokens — OCAP attenuation for goal operations
//!
//! Capabilities gate all goal operations per OCAP principles.
//! Attenuation reduces permissions on delegation (max 7 levels).

use crate::goal::Goal;
use crate::id::{GoalID, WebID};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

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

/// Goal capability token — OCAP authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalCapabilityToken {
    pub id: String,
    pub goal_id: GoalID,
    pub holder_webid: WebID,
    pub operations: Vec<GoalOp>,
    pub expires: DateTime<Utc>,
    pub attenuation_level: u8,
    pub hmac_signature: String,
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
            hmac_signature: String::new(),
        };

        token.hmac_signature = token.compute_hmac(secret);
        token
    }

    fn compute_hmac(&self, secret: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
        mac.update(self.id.as_bytes());
        mac.update(self.goal_id.to_string().as_bytes());
        mac.update(self.holder_webid.to_string().as_bytes());
        mac.update(self.attenuation_level.to_string().as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    pub fn verify_signature(&self, secret: &[u8]) -> bool {
        let expected = self.compute_hmac(secret);
        expected == self.hmac_signature
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires
    }

    pub fn is_valid(&self, secret: &[u8]) -> bool {
        !self.is_expired() && self.verify_signature(secret)
    }

    pub fn can_perform(&self, operation: GoalOp, secret: &[u8]) -> bool {
        self.is_valid(secret) && self.operations.contains(&operation)
    }

    pub fn attenuate(&self, new_operations: Vec<GoalOp>, secret: &[u8]) -> Option<Self> {
        if self.attenuation_level >= 7 {
            return None;
        }

        let mut attenuated =
            GoalCapabilityToken::new(self.goal_id, self.holder_webid, new_operations, secret);
        attenuated.attenuation_level = self.attenuation_level + 1;
        attenuated.expires = Utc::now() + (self.expires - Utc::now()) / 2;
        attenuated.hmac_signature = attenuated.compute_hmac(secret);
        Some(attenuated)
    }
}

/// Goal access control — visibility-based authorization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalAccess {
    Owner,
    Granted,
    Public,
    Denied,
}

impl GoalAccess {
    pub fn check(goal: &Goal, requester_webid: &WebID) -> Self {
        use crate::visibility::Visibility;

        match goal.visibility {
            Visibility::Private => {
                if goal.webid == *requester_webid {
                    GoalAccess::Owner
                } else {
                    GoalAccess::Denied
                }
            }
            Visibility::Shared => {
                if goal.webid == *requester_webid {
                    GoalAccess::Owner
                } else {
                    GoalAccess::Granted
                }
            }
            Visibility::Public => {
                if goal.webid == *requester_webid {
                    GoalAccess::Owner
                } else {
                    GoalAccess::Public
                }
            }
        }
    }

    pub fn can_read(&self) -> bool {
        matches!(
            self,
            GoalAccess::Owner | GoalAccess::Granted | GoalAccess::Public
        )
    }

    pub fn can_write(&self) -> bool {
        matches!(self, GoalAccess::Owner | GoalAccess::Granted)
    }

    pub fn can_admin(&self) -> bool {
        matches!(self, GoalAccess::Owner)
    }
}
