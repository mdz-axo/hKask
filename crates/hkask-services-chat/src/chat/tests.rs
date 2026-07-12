#![allow(clippy::items_after_test_module)]

use super::types::TokenUsage;
use crate::memory::MemoryService;
use hkask_agents::ports::memory_storage::MemoryPortError;
use hkask_agents::ports::memory_storage::RecalledEpisode;
use hkask_agents::ports::{
    EpisodicStoragePort, RecallRequest, RecalledSemantic, SemanticStoragePort, StorageRequest,
};
use hkask_capability::{DelegationAction, DelegationResource, DelegationToken, derive_signing_key};
use hkask_cns::types::loops::ExperienceClassification;
use hkask_types::{Confidence, WebID};
use std::sync::Arc;

#[test]
fn token_usage_gas_cost_one_to_one() {
    let usage = TokenUsage {
        prompt_tokens: 100,
        completion_tokens: 50,
        total_tokens: 150,
    };
    assert_eq!(usage.gas_cost(), 150, "Gas cost must equal total_tokens");
}

#[test]
fn token_usage_zero_tokens_zero_gas() {
    let usage = TokenUsage {
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
    };
    assert_eq!(usage.gas_cost(), 0);
}

#[test]
fn token_usage_gas_uses_total_not_sum_of_parts() {
    let usage = TokenUsage {
        prompt_tokens: 100,
        completion_tokens: 200,
        total_tokens: 250,
    };
    assert_eq!(usage.gas_cost(), 250);
    assert_ne!(usage.gas_cost(), 300);
}

fn test_token(from: WebID, to: WebID) -> DelegationToken {
    DelegationToken::new(
        DelegationResource::Registry,
        "test".into(),
        DelegationAction::Execute,
        from,
        to,
        &derive_signing_key(b"test-hmac-secret-32-bytes-long!!"),
    )
}

struct MockSemanticPort {
    h_mems: Vec<RecalledSemantic>,
}
impl SemanticStoragePort for MockSemanticPort {
    fn store_semantic(
        &self,
        _: StorageRequest,
        _: &DelegationToken,
    ) -> Result<String, MemoryPortError> {
        Ok("id".into())
    }
    fn recall_semantic(&self, _: &RecallRequest) -> Result<Vec<RecalledSemantic>, MemoryPortError> {
        Ok(self.h_mems.clone())
    }
    fn semantic_storage_usage(&self, _: &str) -> Result<usize, MemoryPortError> {
        Ok(self.h_mems.len())
    }
}

struct MockEpisodicPort {
    last_request: std::sync::Mutex<Option<StorageRequest>>,
}
impl EpisodicStoragePort for MockEpisodicPort {
    fn store_episodic(
        &self,
        r: StorageRequest,
        _: &DelegationToken,
    ) -> Result<String, MemoryPortError> {
        *self.last_request.lock().unwrap() = Some(r);
        Ok("id".into())
    }
    fn recall_episodic(&self, _: &RecallRequest) -> Result<Vec<RecalledEpisode>, MemoryPortError> {
        Ok(vec![])
    }
    fn episodic_storage_usage(&self, _: &WebID) -> Result<usize, MemoryPortError> {
        Ok(0)
    }
    fn episodic_storage_budget(&self) -> usize {
        10_000
    }
    fn store_episodic_classified(
        &self,
        r: StorageRequest,
        _: ExperienceClassification,
        _: Option<Confidence>,
        t: &DelegationToken,
    ) -> Result<String, MemoryPortError> {
        self.store_episodic(r, t)
    }
}

#[test]
fn recall_semantic_empty_returns_none() {
    let mock: Arc<MockSemanticPort> = Arc::new(MockSemanticPort { h_mems: vec![] });
    let port: Arc<dyn SemanticStoragePort> = mock;
    let w = WebID::new();
    let result = MemoryService::recall_semantic(&port, "q", &test_token(w, w));
    assert!(result.is_none());
}

#[test]
fn recall_semantic_joins_values_with_newlines() {
    let t = |s: &str| RecalledSemantic {
        id: "x".into(),
        entity: "doc".into(),
        attribute: "c".into(),
        value: serde_json::json!(s),
        confidence: Confidence::new(0.9),
        visibility: hkask_types::Visibility::Shared,
        observed_at: "2026-01-01T00:00:00Z".into(),
        dimension: None,
    };
    let mock: Arc<MockSemanticPort> = Arc::new(MockSemanticPort {
        h_mems: vec![t("A"), t("B")],
    });
    let port: Arc<dyn SemanticStoragePort> = mock;
    let w = WebID::new();
    let result = MemoryService::recall_semantic(&port, "q", &test_token(w, w));
    assert_eq!(result, Some("A\nB".into()));
}

#[test]
fn recall_semantic_filters_non_string_values() {
    let t1 = RecalledSemantic {
        id: "x".into(),
        entity: "doc".into(),
        attribute: "c".into(),
        value: serde_json::json!("Text"),
        confidence: Confidence::new(0.9),
        visibility: hkask_types::Visibility::Shared,
        observed_at: "2026-01-01T00:00:00Z".into(),
        dimension: None,
    };
    let t2 = RecalledSemantic {
        id: "y".into(),
        entity: "doc".into(),
        attribute: "c".into(),
        value: serde_json::json!(42),
        confidence: Confidence::new(0.9),
        visibility: hkask_types::Visibility::Shared,
        observed_at: "2026-01-01T00:00:00Z".into(),
        dimension: None,
    };
    let mock: Arc<MockSemanticPort> = Arc::new(MockSemanticPort {
        h_mems: vec![t1, t2],
    });
    let port: Arc<dyn SemanticStoragePort> = mock;
    let w = WebID::new();
    let result = MemoryService::recall_semantic(&port, "q", &test_token(w, w));
    assert_eq!(result, Some("Text".into()));
}

#[test]
fn store_episodic_records_chat_exchange() {
    let mock: Arc<MockEpisodicPort> = Arc::new(MockEpisodicPort {
        last_request: std::sync::Mutex::new(None),
    });
    let port: Arc<dyn EpisodicStoragePort> = mock.clone();
    let w = WebID::from_persona(b"a");
    MemoryService::store_episodic(&port, "Hello", "Hi!", w, &test_token(w, w), "Agent");
    let req = mock.last_request.lock().unwrap();
    let r = req.as_ref().unwrap();
    assert_eq!(r.entity, "chatted");
    assert_eq!(r.attribute, "chat_turn");
    assert_eq!(r.value["user_input"], "Hello");
    assert_eq!(r.value["agent_response"], "Hi!");
}

#[test]
fn store_episodic_uses_fixed_confidence() {
    let mock: Arc<MockEpisodicPort> = Arc::new(MockEpisodicPort {
        last_request: std::sync::Mutex::new(None),
    });
    let port: Arc<dyn EpisodicStoragePort> = mock.clone();
    let w = WebID::from_persona(b"a");
    MemoryService::store_episodic(&port, "in", "out", w, &test_token(w, w), "Agent");
    let req = mock.last_request.lock().unwrap();
    assert!((req.as_ref().unwrap().confidence.value() - 0.7).abs() < 0.001);
}

#[test]
fn store_episodic_never_panics() {
    let mock: Arc<MockEpisodicPort> = Arc::new(MockEpisodicPort {
        last_request: std::sync::Mutex::new(None),
    });
    let port: Arc<dyn EpisodicStoragePort> = mock;
    let w = WebID::from_persona(b"t");
    MemoryService::store_episodic(&port, "", "", w, &test_token(w, w), "");
}
