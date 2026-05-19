//! Visibility types — OCAP-enforced access control

use serde::{Deserialize, Serialize};

/// Visibility level for artifacts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    #[default]
    Private,
    Public,
    Shared,
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Visibility::Private => "private",
            Visibility::Public => "public",
            Visibility::Shared => "shared",
        }
    }
}

/// OCAP capability for delegation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub resource: String,
    pub action: String,
    pub granted_by: String,
    pub granted_to: String,
}

impl Capability {
    pub fn new(resource: &str, action: &str, granted_by: &str, granted_to: &str) -> Self {
        Self {
            resource: resource.to_string(),
            action: action.to_string(),
            granted_by: granted_by.to_string(),
            granted_to: granted_to.to_string(),
        }
    }
}
