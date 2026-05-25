//! Data Category Grammar — Formal classification for sovereignty boundaries

use crate::{TemplateId, Visibility, WebID};
use serde::{Deserialize, Serialize};

/// Data sovereignty classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DataSovereignty {
    #[default]
    Sovereign,
    Shared,
    Public,
    Custom,
}

/// Data category — Formal classification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum DataCategory {
    Episodic {
        owner: WebID,
        encrypted: bool,
    },
    Semantic {
        scope: Visibility,
    },
    Capability {
        token_id: String,
    },
    Template {
        template_id: TemplateId,
        visibility: Visibility,
    },
    CnsEvent {
        span_category: String,
    },
    AgentPod {
        pod_id: String,
    },
    Custom {
        namespace: String,
        name: String,
    },
}

impl DataCategory {
    pub fn episodic(owner: WebID, encrypted: bool) -> Self {
        Self::Episodic { owner, encrypted }
    }

    pub fn semantic(scope: Visibility) -> Self {
        Self::Semantic { scope }
    }

    pub fn capability(token_id: &str) -> Self {
        Self::Capability {
            token_id: token_id.to_string(),
        }
    }

    pub fn template(template_id: TemplateId, visibility: Visibility) -> Self {
        Self::Template {
            template_id,
            visibility,
        }
    }

    pub fn cns_event(span_category: &str) -> Self {
        Self::CnsEvent {
            span_category: span_category.to_string(),
        }
    }

    pub fn agent_pod(pod_id: &str) -> Self {
        Self::AgentPod {
            pod_id: pod_id.to_string(),
        }
    }

    pub fn custom(namespace: &str, name: &str) -> Self {
        Self::Custom {
            namespace: namespace.to_string(),
            name: name.to_string(),
        }
    }

    pub fn default_sovereignty(&self) -> DataSovereignty {
        match self {
            DataCategory::Episodic { .. } => DataSovereignty::Sovereign,
            DataCategory::Semantic { scope } => match scope {
                Visibility::Private => DataSovereignty::Sovereign,
                Visibility::Shared => DataSovereignty::Shared,
                Visibility::Public => DataSovereignty::Public,
            },
            DataCategory::Capability { .. } => DataSovereignty::Sovereign,
            DataCategory::Template { visibility, .. } => match visibility {
                Visibility::Private => DataSovereignty::Sovereign,
                Visibility::Shared => DataSovereignty::Shared,
                Visibility::Public => DataSovereignty::Public,
            },
            DataCategory::CnsEvent { .. } => DataSovereignty::Shared,
            DataCategory::AgentPod { .. } => DataSovereignty::Shared,
            DataCategory::Custom { .. } => DataSovereignty::Custom,
        }
    }

    pub fn is_sovereign_by_default(&self) -> bool {
        matches!(self.default_sovereignty(), DataSovereignty::Sovereign)
    }

    pub fn owner(&self) -> Option<WebID> {
        match self {
            DataCategory::Episodic { owner, .. } => Some(*owner),
            _ => None,
        }
    }

    pub fn requires_encryption(&self) -> bool {
        match self {
            DataCategory::Episodic { encrypted, .. } => *encrypted,
            DataCategory::Capability { .. } => true,
            _ => false,
        }
    }

    pub fn to_category_string(&self) -> String {
        match self {
            DataCategory::Episodic { .. } => "episodic_memory".to_string(),
            DataCategory::Semantic { .. } => "semantic_memory".to_string(),
            DataCategory::Capability { .. } => "capability_tokens".to_string(),
            DataCategory::Template { .. } => "template_artifacts".to_string(),
            DataCategory::CnsEvent { .. } => "cns_events".to_string(),
            DataCategory::AgentPod { .. } => "agent_pod_artifacts".to_string(),
            DataCategory::Custom { namespace, name } => format!("{}.{}", namespace, name),
        }
    }

    pub fn from_category_string(s: &str) -> Option<Self> {
        match s {
            "episodic_memory" => Some(Self::Episodic {
                owner: WebID::new(),
                encrypted: true,
            }),
            "semantic_memory" => Some(Self::Semantic {
                scope: Visibility::Shared,
            }),
            "capability_tokens" => Some(Self::Capability {
                token_id: String::new(),
            }),
            "template_artifacts" => Some(Self::Template {
                template_id: TemplateId::new(),
                visibility: Visibility::Public,
            }),
            "cns_events" => Some(Self::CnsEvent {
                span_category: String::new(),
            }),
            "agent_pod_artifacts" => Some(Self::AgentPod {
                pod_id: String::new(),
            }),
            _ if s.contains('.') => {
                let parts: Vec<&str> = s.split('.').collect();
                if parts.len() == 2 {
                    Some(Self::Custom {
                        namespace: parts[0].to_string(),
                        name: parts[1].to_string(),
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl std::fmt::Display for DataCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_category_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_category_episodic_sovereignty() {
        let category = DataCategory::episodic(WebID::new(), true);
        assert_eq!(category.default_sovereignty(), DataSovereignty::Sovereign);
        assert!(category.is_sovereign_by_default());
        assert!(category.requires_encryption());
    }

    #[test]
    fn test_data_category_semantic_sovereignty() {
        let private = DataCategory::semantic(Visibility::Private);
        assert_eq!(private.default_sovereignty(), DataSovereignty::Sovereign);

        let shared = DataCategory::semantic(Visibility::Shared);
        assert_eq!(shared.default_sovereignty(), DataSovereignty::Shared);

        let public = DataCategory::semantic(Visibility::Public);
        assert_eq!(public.default_sovereignty(), DataSovereignty::Public);
    }

    #[test]
    fn test_data_category_capability_sovereignty() {
        let category = DataCategory::capability("test-token");
        assert_eq!(category.default_sovereignty(), DataSovereignty::Sovereign);
        assert!(category.requires_encryption());
    }

    #[test]
    fn test_data_category_string_conversion() {
        let category = DataCategory::episodic(WebID::new(), true);
        let string = category.to_category_string();
        assert_eq!(string, "episodic_memory");

        let parsed = DataCategory::from_category_string(&string);
        assert!(parsed.is_some());
    }

    #[test]
    fn test_data_category_custom() {
        let category = DataCategory::custom("test", "data");
        assert_eq!(category.default_sovereignty(), DataSovereignty::Custom);
        assert_eq!(category.to_category_string(), "test.data");
    }

    #[test]
    fn test_data_category_owner() {
        let owner = WebID::new();
        let episodic = DataCategory::episodic(owner, true);
        assert_eq!(episodic.owner(), Some(owner));

        let semantic = DataCategory::semantic(Visibility::Public);
        assert_eq!(semantic.owner(), None);
    }
}
