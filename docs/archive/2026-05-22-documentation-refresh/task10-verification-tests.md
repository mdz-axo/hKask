# GML Verification Test Suite

**Type:** Test Specification  
**Version:** 0.1.0  
**Priority:** Medium

---

## Overview

Comprehensive test suite for all GML operations, templates, and infrastructure.

---

## Test Categories

| Category | Tests | Purpose |
|----------|-------|---------|
| Schema validation | 10 | Reject invalid inputs |
| Capability enforcement | 8 | Block unauthorized ops |
| MWC computation | 12 | Verify mathematical correctness |
| Error handling | 10 | Graceful failure |
| CNS instrumentation | 5 | Spans emitted correctly |
| End-to-end cascade | 4 | Full FlowDef execution |

**Total:** 49 tests

---

## Schema Validation Tests

```rust
#[cfg(test)]
mod schema_validation_tests {
    use super::*;
    
    #[test]
    fn test_valid_concept() {
        let concept = load_fixture("freedom-concept.json");
        let result = validate_concept(&concept);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_missing_name() {
        let concept = json!({"t_state": {...}, "r_state": {...}, "l": 100.0, "ports": [...]});
        let result = validate_concept(&concept);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field, "name");
    }
    
    #[test]
    fn test_missing_t_state() {
        let concept = load_fixture("freedom-concept.json");
        // Remove t_state
        let result = validate_concept(&concept);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_invalid_l_zero() {
        let mut concept = load_fixture("freedom-concept.json");
        concept["l"] = json!(0.0);
        let result = validate_concept(&concept);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("> 0"));
    }
    
    #[test]
    fn test_invalid_l_negative() {
        let mut concept = load_fixture("freedom-concept.json");
        concept["l"] = json!(-10.0);
        let result = validate_concept(&concept);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_no_ports() {
        let mut concept = load_fixture("freedom-concept.json");
        concept["ports"] = json!([]);
        let result = validate_concept(&concept);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_invalid_port_affinity() {
        let mut concept = load_fixture("freedom-concept.json");
        concept["ports"][0]["affinity_c"] = json!(0.0);
        let result = validate_concept(&concept);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_invalid_current_alpha_negative() {
        let mut concept = load_fixture("freedom-concept.json");
        concept["current_alpha"] = json!(-1.0);
        let result = validate_concept(&concept);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_invalid_r_bar_out_of_range() {
        let mut concept = load_fixture("freedom-concept.json");
        concept["current_r_bar"] = json!(1.5);
        let result = validate_concept(&concept);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_effector_missing_concentration() {
        let effector = json!({"name": "test", "effect_type": "Activator", "shape": "SecurityThreat"});
        let result = validate_effector(&effector);
        assert!(result.is_err());
    }
}
```

---

## Capability Enforcement Tests

```rust
#[cfg(test)]
mod capability_enforcement_tests {
    use super::*;
    
    #[test]
    fn test_valid_capability() {
        let capability = load_capability("capability-valid.json");
        let middleware = CapabilityMiddleware::new();
        let result = middleware.authorize_and_render(
            &template,
            &inputs,
            &capability,
        );
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_operation_not_allowed() {
        let capability = load_capability("capability-no-bind.json");
        let middleware = CapabilityMiddleware::new();
        let inputs = BindInputs { ... };
        let result = middleware.authorize_and_render(
            &template,
            &inputs,
            &capability,
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CapabilityError::OperationNotAllowed(GmlOperation::Bind)));
    }
    
    #[test]
    fn test_budget_exceeded() {
        let mut capability = load_capability("capability-valid.json");
        capability.effector_budget = Some(5.0);
        
        let middleware = CapabilityMiddleware::new();
        let inputs = BindInputs {
            effectors: vec![Effector { concentration: 10.0, ... }],
            ...
        };
        let result = middleware.authorize_and_render(
            &template,
            &inputs,
            &capability,
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CapabilityError::BudgetExceeded { .. }));
    }
    
    #[test]
    fn test_concept_not_allowed() {
        let mut capability = load_capability("capability-valid.json");
        capability.concepts_allowed = vec![ConceptId::new("gml:concept:privacy")];
        
        let middleware = CapabilityMiddleware::new();
        let inputs = RecognizeInputs {
            concept: load_fixture("freedom-concept.json"),
            ...
        };
        let result = middleware.authorize_and_render(
            &template,
            &inputs,
            &capability,
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CapabilityError::ConceptNotAllowed(_)));
    }
    
    #[test]
    fn test_invalid_signature() {
        let mut capability = load_capability("capability-valid.json");
        // Corrupt signature
        capability.signature = Signature::from_bytes([0u8; 64]);
        
        let middleware = CapabilityMiddleware::new();
        let result = middleware.authorize_and_render(
            &template,
            &inputs,
            &capability,
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CapabilityError::InvalidSignature));
    }
    
    #[test]
    fn test_expired_capability() {
        let mut capability = load_capability("capability-valid.json");
        capability.valid_until = Some(chrono::Utc::now().timestamp() - 3600);  // 1 hour ago
        
        let middleware = CapabilityMiddleware::new();
        let result = middleware.authorize_and_render(
            &template,
            &inputs,
            &capability,
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CapabilityError::Expired));
    }
    
    #[test]
    fn test_not_yet_valid() {
        let mut capability = load_capability("capability-valid.json");
        capability.valid_from = Some(chrono::Utc::now().timestamp() + 3600);  // 1 hour in future
        
        let middleware = CapabilityMiddleware::new();
        let result = middleware.authorize_and_render(
            &template,
            &inputs,
            &capability,
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CapabilityError::NotYetValid));
    }
    
    #[test]
    fn test_attenuated_capability() {
        let parent = load_capability("capability-valid.json");
        let child = parent.attenuate(new_issuer, new_subject);
        
        // Child should have subset of operations
        assert!(child.operations.len() <= parent.operations.len());
        for op in &child.operations {
            assert!(parent.operations.contains(op));
        }
    }
}
```

---

## MWC Computation Tests

```rust
#[cfg(test)]
mod mwc_computation_tests {
    use super::*;
    
    #[test]
    fn test_state_function_no_ligand() {
        let algebra = GmlAlgebraImpl;
        // L = 100, α = 0 → R̄ = 1/(1+L) = 0.01
        let r_bar = algebra.state_function(100.0, 0.1, 4, 0.0);
        assert!((r_bar - 0.01).abs() < 0.001);
    }
    
    #[test]
    fn test_state_function_saturating() {
        let algebra = GmlAlgebraImpl;
        // L = 100, c = 0.1, α = 100 → R̄ ≈ 0.91
        let r_bar = algebra.state_function(100.0, 0.1, 4, 100.0);
        assert!(r_bar > 0.9);
    }
    
    #[test]
    fn test_state_function_strong_activator() {
        let algebra = GmlAlgebraImpl;
        // L = 1000, c = 0.01, α = 10 → R̄ > 0.5
        let r_bar = algebra.state_function(1000.0, 0.01, 4, 10.0);
        assert!(r_bar > 0.5);
    }
    
    #[test]
    fn test_state_function_strong_inhibitor() {
        let algebra = GmlAlgebraImpl;
        // L = 0.1, c = 10.0, α = 10 → R̄ < 0.1
        let r_bar = algebra.state_function(0.1, 10.0, 4, 10.0);
        assert!(r_bar < 0.1);
    }
    
    #[test]
    fn test_hill_coefficient_positive_cooperativity() {
        let algebra = GmlAlgebraImpl;
        // n = 4, c = 0.01, α = 1 → n_H > 1
        let n_h = algebra.hill_coefficient(4, 0.01, 1.0);
        assert!(n_h > 1.0);
    }
    
    #[test]
    fn test_hill_coefficient_negative_cooperativity() {
        let algebra = GmlAlgebraImpl;
        // n = 1, c = 2.0, α = 0.1 → n_H < 0
        let n_h = algebra.hill_coefficient(1, 2.0, 0.1);
        assert!(n_h < 0.0);
    }
    
    #[test]
    fn test_hill_coefficient_no_cooperativity() {
        let algebra = GmlAlgebraImpl;
        // n = 1, c = 1.0, α = 1 → n_H = 0
        let n_h = algebra.hill_coefficient(1, 1.0, 1.0);
        assert!((n_h - 0.0).abs() < 0.001);
    }
    
    #[test]
    fn test_partition_function_positive() {
        let algebra = GmlAlgebraImpl;
        let z = algebra.partition_function(100.0, 0.1, 4, 1.0);
        assert!(z > 0.0);
    }
    
    #[test]
    fn test_binding_function_range() {
        let algebra = GmlAlgebraImpl;
        let y_bar = algebra.binding_function(100.0, 0.1, 4, 1.0);
        assert!(y_bar >= 0.0 && y_bar <= 1.0);
    }
    
    #[test]
    fn test_equilibrium_distribution() {
        let algebra = GmlAlgebraImpl;
        let dist = algebra.equilibrium(100.0, 0.1, 4, 1.0);
        
        assert!(dist.p_r >= 0.0 && dist.p_r <= 1.0);
        assert!(dist.p_t >= 0.0 && dist.p_t <= 1.0);
        assert!((dist.p_r + dist.p_t - 1.0).abs() < 0.001);
    }
    
    #[test]
    fn test_boltzmann_factor_consistency() {
        let e_t = -10.0;
        let e_r = -5.0;
        let kt = 1.0;
        
        let l_from_boltzmann = boltzmann_factor(e_t, e_r, kt);
        let l_expected = ((e_t - e_r) / kt).exp();
        
        assert!((l_from_boltzmann - l_expected).abs() < 0.001);
    }
    
    #[test]
    fn test_energy_from_l_consistency() {
        let l = 100.0;
        let kt = 1.0;
        
        let delta_e = energy_from_l(l, kt);
        let l_recovered = l_from_energy(delta_e, kt);
        
        assert!((l - l_recovered).abs() < 0.001);
    }
}
```

---

## Error Handling Tests

```rust
#[cfg(test)]
mod error_handling_tests {
    use super::*;
    
    #[test]
    fn test_missing_input_error() {
        let template = get_template("gml/recognize-ensemble");
        let inputs = TemplateInputs::new();  // Empty inputs
        let result = template.render(&inputs);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing"));
    }
    
    #[test]
    fn test_validation_error_shows_fields() {
        let template = get_template("gml/error-validation");
        let inputs = TemplateInputs::new()
            .with_field_errors(vec![
                FieldError { field: "l", message: "L must be > 0" },
                FieldError { field: "ports", message: "At least one port required" },
            ]);
        let output = template.render(&inputs).unwrap();
        
        assert!(output.contains("l"));
        assert!(output.contains("L must be > 0"));
        assert!(output.contains("ports"));
    }
    
    #[test]
    fn test_generic_error_with_context() {
        let template = get_template("gml/error-generic");
        let inputs = TemplateInputs::new()
            .with_error_code("GML_CAPABILITY_DENIED")
            .with_message("Operation not allowed")
            .with_context(map!{
                "operation" => "bind",
                "scope" => "Private",
            });
        let output = template.render(&inputs).unwrap();
        
        assert!(output.contains("GML_CAPABILITY_DENIED"));
        assert!(output.contains("Operation not allowed"));
        assert!(output.contains("bind"));
    }
    
    #[test]
    fn test_no_compatible_port_error() {
        let template = get_template("gml/bind-effector");
        let inputs = BindInputs {
            concept: load_fixture("freedom-concept.json"),
            effectors: vec![Effector {
                shape: "UnknownShape".to_string(),
                ...
            }],
        };
        let output = template.render(&inputs).unwrap();
        
        assert!(output.contains("No compatible ports found"));
        assert!(output.contains("SecurityThreat"));  // Available port shape
    }
    
    #[test]
    fn test_budget_exceeded_error_shows_values() {
        let template = get_template("gml/error-generic");
        let inputs = TemplateInputs::new()
            .with_error_code("GML_BUDGET_EXCEEDED")
            .with_context(map!{
                "requested" => "100.0",
                "allowed" => "50.0",
            });
        let output = template.render(&inputs).unwrap();
        
        assert!(output.contains("100.0"));
        assert!(output.contains("50.0"));
    }
    
    #[test]
    fn test_template_graceful_degradation() {
        // Template should handle missing optional fields
        let concept = json!({
            "name": "Freedom",
            "t_state": {"description": "...", "energy": -10.0},
            "r_state": {"description": "...", "energy": -5.0},
            "l": 100.0,
            "ports": [{"name": "test", "effector_shape": "Test", "affinity_c": 0.1}],
            // current_alpha missing - should use default
        });
        
        let template = get_template("gml/recognize-ensemble");
        let result = template.render(&TemplateInputs::with_concept(concept));
        
        assert!(result.is_ok());  // Should not error
    }
    
    #[test]
    fn test_cns_error_event_emitted() {
        let emitter = MockCnsEmitter::new();
        let mut adapter = CnsAdapter::new(Box::new(emitter.clone()));
        
        adapter.record_error("GML_TEST_ERROR", "Test error message").unwrap();
        
        let events = emitter.get_events();
        assert_eq!(events.len(), 1);
        
        match &events[0] {
            CnsEvent::Error { error_code, message } => {
                assert_eq!(error_code, "GML_TEST_ERROR");
                assert_eq!(message, "Test error message");
            }
            _ => panic!("Expected Error event"),
        }
    }
    
    #[test]
    fn test_span_error_status() {
        let emitter = MockCnsEmitter::new();
        let mut adapter = CnsAdapter::new(Box::new(emitter.clone()));
        
        adapter.start_span("test", map!{}).unwrap();
        adapter.end_span(SpanStatus::Error, map!{"error" => "test"}).unwrap();
        
        let events = emitter.get_events();
        
        match &events[1] {
            CnsEvent::SpanEnd { status, .. } => {
                assert_eq!(*status, SpanStatus::Error);
            }
            _ => panic!("Expected SpanEnd"),
        }
    }
    
    #[test]
    fn test_audit_log_on_state_change() {
        let emitter = MockCnsEmitter::new();
        let mut adapter = CnsAdapter::new(Box::new(emitter.clone()));
        
        let entry = AuditEntry {
            operation: "bind".to_string(),
            concept_id: ConceptId::new("test"),
            capability_hash: "hash".to_string(),
            before_r_bar: Some(0.1),
            after_r_bar: Some(0.6),
            result: "success".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        adapter.audit_log(entry).unwrap();
        
        let events = emitter.get_events();
        assert!(events.iter().any(|e| matches!(e, CnsEvent::AuditLog(_))));
    }
    
    #[test]
    fn test_variety_counter_algedonic_alert() {
        let emitter = MockCnsEmitter::new();
        emitter.set_variety("gml.test", 150);  // Above threshold of 100
        
        let adapter = CnsAdapter::new(Box::new(emitter.clone()));
        let needs_alert = adapter.check_variety("gml.test", 100).unwrap();
        
        assert!(needs_alert);
    }
}
```

---

## CNS Instrumentation Tests

```rust
#[cfg(test)]
mod cns_instrumentation_tests {
    use super::*;
    
    #[test]
    fn test_span_start_emitted() {
        let emitter = MockCnsEmitter::new();
        let mut adapter = CnsAdapter::new(Box::new(emitter.clone()));
        
        adapter.start_span("recognize", map!{"concept_id" => "test"}).unwrap();
        
        let events = emitter.get_events();
        assert_eq!(events.len(), 1);
        
        match &events[0] {
            CnsEvent::SpanStart { span, .. } => {
                assert_eq!(span, "cns.gml.recognize");
            }
            _ => panic!("Expected SpanStart"),
        }
    }
    
    #[test]
    fn test_span_end_emitted() {
        let emitter = MockCnsEmitter::new();
        let mut adapter = CnsAdapter::new(Box::new(emitter.clone()));
        
        adapter.start_span("test", map!{}).unwrap();
        adapter.end_span(SpanStatus::Ok, map!{}).unwrap();
        
        let events = emitter.get_events();
        assert_eq!(events.len(), 2);  // Start + End
        
        match &events[1] {
            CnsEvent::SpanEnd { status, .. } => {
                assert_eq!(*status, SpanStatus::Ok);
            }
            _ => panic!("Expected SpanEnd"),
        }
    }
    
    #[test]
    fn test_span_stack_nested() {
        let emitter = MockCnsEmitter::new();
        let mut adapter = CnsAdapter::new(Box::new(emitter.clone()));
        
        adapter.start_span("outer", map!{}).unwrap();
        adapter.start_span("inner", map!{}).unwrap();
        adapter.end_span(SpanStatus::Ok, map!{}).unwrap();
        adapter.end_span(SpanStatus::Ok, map!{}).unwrap();
        
        let events = emitter.get_events();
        assert_eq!(events.len(), 4);  // 2 Start + 2 End
        
        // Verify parent span linkage
        match &events[1] {
            CnsEvent::SpanStart { parent_span, .. } => {
                assert!(parent_span.is_some());
            }
            _ => panic!("Expected nested span"),
        }
    }
    
    #[test]
    fn test_audit_log_format() {
        let emitter = MockCnsEmitter::new();
        let mut adapter = CnsAdapter::new(Box::new(emitter.clone()));
        
        let entry = AuditEntry {
            operation: "bind".to_string(),
            concept_id: ConceptId::new("gml:concept:freedom"),
            capability_hash: "abc123".to_string(),
            before_r_bar: Some(0.1),
            after_r_bar: Some(0.6),
            result: "success".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        adapter.audit_log(entry).unwrap();
        
        let events = emitter.get_events();
        
        match &events[0] {
            CnsEvent::AuditLog(entry) => {
                assert_eq!(entry.operation, "bind");
                assert_eq!(entry.before_r_bar, Some(0.1));
                assert_eq!(entry.after_r_bar, Some(0.6));
            }
            _ => panic!("Expected AuditLog"),
        }
    }
    
    #[test]
    fn test_algedonic_alert_emitted() {
        let emitter = MockCnsEmitter::new();
        emitter.set_variety("gml.assess", 150);
        
        let adapter = CnsAdapter::new(Box::new(emitter.clone()));
        let needs_alert = adapter.check_variety("gml.assess", 100).unwrap();
        
        assert!(needs_alert);
    }
}
```

---

## End-to-End Cascade Tests

```rust
#[cfg(test)]
mod end_to_end_tests {
    use super::*;
    
    #[test]
    fn test_full_recognize_cascade() {
        let handler = GmlMcpHandler::new();
        let capability = load_capability("capability-valid.json");
        let inputs = RecognizeInputs {
            concept: load_fixture("freedom-concept.json"),
        };
        
        let output = handler.handle_recognize(inputs, capability).unwrap();
        
        assert!(output.contains("Allosteric Recognition: Freedom"));
        assert!(output.contains("T-State"));
        assert!(output.contains("R-State"));
        assert!(output.contains("Current R̄"));
    }
    
    #[test]
    fn test_full_bind_cascade() {
        let handler = GmlMcpHandler::new();
        let capability = load_capability("capability-valid.json");
        let inputs = BindInputs {
            concept: load_fixture("freedom-concept.json"),
            effectors: vec![load_effector("security-crisis.json")],
        };
        
        let output = handler.handle_bind(inputs, capability).unwrap();
        
        assert!(output.contains("Effector Binding Analysis"));
        assert!(output.contains("Equilibrium Shift"));
        assert!(output.contains("Δ"));  // Delta symbol
    }
    
    #[test]
    fn test_full_cascade_with_network() {
        let handler = GmlMcpHandler::new();
        let capability = load_capability("capability-valid.json");
        let inputs = AssessInputs {
            shift: load_shift("freedom-shift.json"),
            network: Some(load_network("small-network.json")),
        };
        
        let output = handler.handle_assess(inputs, capability).unwrap();
        
        assert!(output.contains("Coherence Assessment"));
        assert!(output.contains("Network Homeostasis"));
        assert!(output.contains("Mean coherence"));
    }
    
    #[test]
    fn test_cascade_emits_cns_spans() {
        let emitter = MockCnsEmitter::new();
        let handler = GmlMcpHandler::new_with_emitter(Box::new(emitter.clone()));
        let capability = load_capability("capability-valid.json");
        
        let inputs = RecognizeInputs {
            concept: load_fixture("freedom-concept.json"),
        };
        
        handler.handle_recognize(inputs, capability).unwrap();
        
        let events = emitter.get_events();
        
        // Should have: span_start, audit_log, span_end
        assert!(events.iter().any(|e| matches!(e, CnsEvent::SpanStart { .. })));
        assert!(events.iter().any(|e| matches!(e, CnsEvent::AuditLog(_))));
        assert!(events.iter().any(|e| matches!(e, CnsEvent::SpanEnd { .. })));
    }
}
```

---

## Test Fixtures

```
hkask-testing/gml/fixtures/
├── freedom-concept.json
├── privacy-concept.json
├── intelligence-concept.json
├── effectors/
│   ├── security-crisis.json
│   └── economic-pressure.json
├── capabilities/
│   ├── capability-valid.json
│   ├── capability-expired.json
│   ├── capability-no-bind.json
│   └── capability-no-budget.json
├── networks/
│   └── small-network.json
└── expected-outputs/
    ├── freedom-recognize.md
    ├── freedom-bind.md
    └── freedom-assess.md
```

---

## Running Tests

```bash
# Run all GML tests
cargo test -p hkask-testing -- gml

# Run specific category
cargo test -p hkask-testing -- gml::schema_validation_tests
cargo test -p hkask-testing -- gml::capability_enforcement_tests
cargo test -p hkask-testing -- gml::mwc_computation_tests

# Run with coverage
cargo llvm-cov -p hkask-testing -- gml

# Generate test report
cargo test -p hkask-testing -- gml -- --format json > gml-test-results.json
```

---

*ℏKask — Planck's Constant of Agent Systems — GML v0.1.0*
*Task 10 complete: Verification test suite specified.*
