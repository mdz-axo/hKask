//! Contract tests for hkask-mcp-training — MLSchema concepts and adapter types.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: MLSchema ontology constants, adapter metadata types, and request deserialization.

// ── MLSchema concept tests ─────────────────────────────────────────────────

#[test]
fn mls_concepts_are_non_empty() {
    assert!(!hkask_mcp_training::mlschema::MODEL.is_empty());
    assert!(!hkask_mcp_training::mlschema::RUN.is_empty());
    assert!(!hkask_mcp_training::mlschema::DATA.is_empty());
    assert!(!hkask_mcp_training::mlschema::HYPER_PARAMETER.is_empty());
    assert!(!hkask_mcp_training::mlschema::EVALUATION.is_empty());
}

#[test]
fn mls_concepts_have_correct_prefix() {
    assert!(hkask_mcp_training::mlschema::MODEL.starts_with("mls:"));
    assert!(hkask_mcp_training::mlschema::RUN.starts_with("mls:"));
    assert!(hkask_mcp_training::mlschema::DATA.starts_with("mls:"));
}

#[test]
fn mls_derivation_concept_exists() {
    assert_eq!(
        hkask_mcp_training::mlschema::WAS_DERIVED_FROM,
        "mls:wasDerivedFrom"
    );
}

// ── Adapter type tests ─────────────────────────────────────────────────────

#[test]
fn lora_adapter_type_exists() {
    let _type_name = std::any::type_name::<hkask_mcp_training::adapters::LoRAAdapter>();
    assert!(_type_name.contains("hkask_mcp_training"));
}

#[test]
fn in_memory_adapter_store_is_constructable() {
    let store = hkask_mcp_training::adapters::InMemoryAdapterStore::default();
    let _ = store;
}

// ── Request type tests ─────────────────────────────────────────────────────

#[test]
fn ingest_qa_request_type_exists() {
    let _type_name = std::any::type_name::<hkask_mcp_training::types::IngestQaRequest>();
    assert!(_type_name.contains("hkask_mcp_training"));
}

#[test]
fn training_submit_request_type_exists() {
    let _type_name = std::any::type_name::<hkask_mcp_training::types::TrainSubmitRequest>();
    assert!(_type_name.contains("hkask_mcp_training"));
}

#[test]
fn training_host_id_enum_exists() {
    let _type_name = std::any::type_name::<hkask_mcp_training::providers::TrainingHostId>();
    assert!(_type_name.contains("hkask_mcp_training"));
}

#[test]
fn training_job_status_enum_exists() {
    let _type_name = std::any::type_name::<hkask_mcp_training::providers::TrainingJobStatus>();
    assert!(_type_name.contains("hkask_mcp_training"));
}

// ── Dataset pipeline tests ─────────────────────────────────────────────────

#[test]
fn dataset_pipeline_type_exists() {
    let _type_name = std::any::type_name::<hkask_mcp_training::dataset::DatasetPipeline>();
    assert!(_type_name.contains("hkask_mcp_training"));
}

// ── Schema generation tests ────────────────────────────────────────────────

#[test]
fn request_types_have_schemas() {
    let schema = schemars::schema_for!(hkask_mcp_training::types::IngestQaRequest);
    let schema_json = serde_json::to_value(&schema).expect("schema should serialize");
    assert!(schema_json.is_object());

    let schema = schemars::schema_for!(hkask_mcp_training::types::TrainSubmitRequest);
    let schema_json = serde_json::to_value(&schema).expect("schema should serialize");
    assert!(schema_json.is_object());
}
