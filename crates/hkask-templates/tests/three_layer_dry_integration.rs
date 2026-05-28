//! Integration test for the three-layer DRY system
//!
//! Tests the full pipeline: memory recall → session dedup → context assembly

use hkask_ensemble::chat_dedup::{SessionDedup, extract_context_window};
use hkask_memory::recall_dedup::{dedup_triples, eav_hash};
use hkask_storage::Triple;
use hkask_templates::adapters::StubMemoryPort;
use hkask_templates::context_assembly::{ContextAssembler, ContextFragment, FragmentSource};
use hkask_types::WebID;
use serde_json::json;
use uuid::Uuid;

fn test_webid() -> WebID {
    WebID(Uuid::new_v4())
}

#[test]
fn test_three_layer_pipeline_no_duplicates() {
    // Layer 1: Memory recall dedup
    let triples = vec![
        Triple::new("Paris", "capital_of", json!("France"), test_webid()),
        Triple::new("Berlin", "capital_of", json!("Germany"), test_webid()),
        Triple::new("Paris", "capital_of", json!("France"), test_webid()), // duplicate
    ];

    let deduped_triples = dedup_triples(triples);
    assert_eq!(deduped_triples.len(), 2);

    // Layer 2: Session message dedup
    let messages = vec![
        "Hello world".to_string(),
        "How are you?".to_string(),
        "Hello world".to_string(), // duplicate
    ];

    let mut session_dedup = SessionDedup::new(100);
    let deduped_messages = extract_context_window(&messages, 1000, &mut session_dedup);
    assert_eq!(deduped_messages.len(), 2);

    // Layer 3: Context assembly dedup
    let mut assembler = ContextAssembler::new(4096);

    // Add system prompt
    assembler.add(ContextFragment::new(
        "You are a helpful assistant.".into(),
        FragmentSource::System,
    ));

    // Add user message
    assembler.add(ContextFragment::new(
        "What is the capital of France?".into(),
        FragmentSource::User,
    ));

    // Add memory context (deduped in Layer 1)
    for triple in &deduped_triples {
        assembler.add(ContextFragment::new(
            format!("{}: {} = {}", triple.entity, triple.attribute, triple.value),
            FragmentSource::SemanticMemory,
        ));
    }

    // Add session history (deduped in Layer 2)
    for message in &deduped_messages {
        assembler.add(ContextFragment::new(
            message.clone(),
            FragmentSource::SessionHistory,
        ));
    }

    // Try to add duplicate user message (should be rejected)
    let result = assembler.add(ContextFragment::new(
        "What is the capital of France?".into(),
        FragmentSource::User,
    ));
    assert_eq!(
        result,
        hkask_templates::context_assembly::AddResult::DuplicateExact
    );

    // Verify stats
    let stats = assembler.stats();
    assert_eq!(stats.fragments_offered, 7); // 1 system + 1 user + 2 memory + 2 history + 1 duplicate
    assert_eq!(stats.fragments_accepted, 6); // 1 system + 1 user + 2 memory + 2 history
    assert_eq!(stats.duplicates_exact, 1);

    // Render and verify
    let prompt = assembler.render();
    assert!(prompt.contains("[SYSTEM] You are a helpful assistant."));
    assert!(prompt.contains("[USER] What is the capital of France?"));
    assert!(prompt.contains("[MEMORY] Paris: capital_of = \"France\""));
    assert!(prompt.contains("[MEMORY] Berlin: capital_of = \"Germany\""));
    assert!(prompt.contains("[HISTORY] Hello world"));
    assert!(prompt.contains("[HISTORY] How are you?"));
}

#[test]
fn test_three_layer_pipeline_with_stub_memory() {
    // Use StubMemoryPort (returns empty results)
    let stub_memory = StubMemoryPort;

    // Layer 1: Memory recall (stub returns empty)
    let semantic_fragments = stub_memory.query_semantic("test").unwrap();
    assert!(semantic_fragments.is_empty());

    // Layer 2: Session message dedup
    let messages = vec!["Message 1".to_string(), "Message 2".to_string()];

    let mut session_dedup = SessionDedup::new(100);
    let deduped_messages = extract_context_window(&messages, 1000, &mut session_dedup);
    assert_eq!(deduped_messages.len(), 2);

    // Layer 3: Context assembly
    let mut assembler = ContextAssembler::new(4096);

    assembler.add(ContextFragment::new(
        "System prompt".into(),
        FragmentSource::System,
    ));

    for message in &deduped_messages {
        assembler.add(ContextFragment::new(
            message.clone(),
            FragmentSource::SessionHistory,
        ));
    }

    assert_eq!(assembler.len(), 3); // 1 system + 2 history
}

#[test]
fn test_eav_hash_determinism() {
    let t1 = Triple::new("Paris", "capital_of", json!("France"), test_webid());
    let t2 = Triple::new("Paris", "capital_of", json!("France"), test_webid());

    let h1 = eav_hash(&t1);
    let h2 = eav_hash(&t2);

    assert_eq!(h1, h2);
}

#[test]
fn test_context_assembly_budget_enforcement() {
    let mut assembler = ContextAssembler::new(10); // Very small budget

    assembler.add(ContextFragment::new("Short".into(), FragmentSource::User));

    let result = assembler.add(ContextFragment::new(
        "This is way too long for the budget".into(),
        FragmentSource::User,
    ));

    assert_eq!(
        result,
        hkask_templates::context_assembly::AddResult::BudgetExceeded
    );
    assert_eq!(assembler.len(), 1);
}
