# GML CNS Adapter Implementation

**Type:** Infrastructure Adapter  
**Version:** 0.1.0  
**Priority:** Medium

---

## Overview

Implement CNS instrumentation as an adapter (outbound port) rather than Jinja2 macros, per hexagonal architecture.

---

## Current State (Before)

```jinja2
{# Template emits CNS events directly #}
{{ cns.emit({"event": "span_start", "span": "cns.gml.recognize", ...}) }}
```

**Problems:**
- Templates coupled to CNS infrastructure
- Hard to test CNS emission
- No abstraction boundary

---

## Target State (After)

```rust
// Infrastructure adapter
pub struct CnsAdapter {
    emitter: Box<dyn CnsEmitter>,
    span_stack: Vec<SpanId>,
}

impl CnsAdapter {
    pub fn start_span(&mut self, name: &str, attributes: Map) -> SpanId;
    pub fn end_span(&mut self, id: SpanId, status: Status);
    pub fn record_error(&mut self, id: SpanId, error: Error);
    pub fn audit_log(&mut self, entry: AuditEntry);
}
```

```jinja2
{# Template calls adapter #}
{% do cns_adapter.start_span('recognize', {'concept_id': concept.id}) %}
```

---

## Adapter Interface

```rust
/// CNS emitter interface (outbound port)
pub trait CnsEmitter: Send + Sync {
    /// Emit event to CNS
    fn emit(&self, event: CnsEvent) -> Result<(), CnsError>;
    
    /// Get variety counter value
    fn get_variety(&self, counter_name: &str) -> u64;
}

/// CNS adapter for templates
pub struct CnsAdapter {
    emitter: Box<dyn CnsEmitter>,
    span_stack: Vec<SpanId>,
    current_concept: Option<ConceptId>,
}

impl CnsAdapter {
    pub fn new(emitter: Box<dyn CnsEmitter>) -> Self {
        Self {
            emitter,
            span_stack: Vec::new(),
            current_concept: None,
        }
    }
    
    /// Start a span
    pub fn start_span(&mut self, operation: &str, attributes: Map) -> Result<SpanId, CnsError> {
        let span_id = SpanId::new();
        
        let event = CnsEvent::SpanStart {
            span: format!("cns.gml.{}", operation),
            span_id: span_id.clone(),
            parent_span: self.span_stack.last().cloned(),
            attributes,
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        self.emitter.emit(event)?;
        self.span_stack.push(span_id.clone());
        
        Ok(span_id)
    }
    
    /// End a span
    pub fn end_span(&mut self, status: SpanStatus, attributes: Map) -> Result<(), CnsError> {
        let span_id = self.span_stack.pop().ok_or(CnsError::NoActiveSpan)?;
        
        let event = CnsEvent::SpanEnd {
            span_id,
            status,
            attributes,
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        self.emitter.emit(event)
    }
    
    /// Record error
    pub fn record_error(&mut self, error_code: &str, message: &str) -> Result<(), CnsError> {
        let event = CnsEvent::Error {
            error_code: error_code.to_string(),
            message: message.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        self.emitter.emit(event)
    }
    
    /// Check variety counter
    pub fn check_variety(&self, counter_name: &str, threshold: u64) -> Result<bool, CnsError> {
        let variety = self.emitter.get_variety(counter_name);
        Ok(variety > threshold)
    }
    
    /// Audit log entry
    pub fn audit_log(&mut self, entry: AuditEntry) -> Result<(), CnsError> {
        let event = CnsEvent::AuditLog(entry);
        self.emitter.emit(event)
    }
}
```

---

## Event Types

```rust
/// CNS event types
#[derive(Debug, Clone)]
pub enum CnsEvent {
    /// Start span
    SpanStart {
        span: String,
        span_id: SpanId,
        parent_span: Option<SpanId>,
        attributes: Map,
        timestamp: i64,
    },
    
    /// End span
    SpanEnd {
        span_id: SpanId,
        status: SpanStatus,
        attributes: Map,
        timestamp: i64,
    },
    
    /// Error event
    Error {
        error_code: String,
        message: String,
        timestamp: i64,
    },
    
    /// Audit log entry
    AuditLog(AuditEntry),
    
    /// Algedonic alert
    AlgedonicAlert {
        alert: String,
        counter: String,
        value: u64,
        threshold: u64,
        level: AlertLevel,
        timestamp: i64,
    },
}

/// Span status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpanStatus {
    Ok,
    Error,
}

/// Audit entry
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub operation: String,
    pub concept_id: ConceptId,
    pub capability_hash: String,
    pub before_r_bar: Option<f64>,
    pub after_r_bar: Option<f64>,
    pub result: String,
    pub timestamp: i64,
}

/// Alert level
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}
```

---

## Template Integration

```rust
// hkask-mcp-gml/src/template_context.rs

use hkask_cns::CnsAdapter;

/// Context provided to Jinja2 templates
pub struct TemplateContext {
    pub gml_algebra: Box<dyn GmlAlgebraService>,
    pub cns_adapter: CnsAdapter,
}

impl TemplateContext {
    pub fn new(algebra: Box<dyn GmlAlgebraService>, cns: CnsAdapter) -> Self {
        Self {
            gml_algebra: algebra,
            cns_adapter: cns,
        }
    }
    
    /// Register with Jinja2 engine
    pub fn register(&self, env: &mut Environment) {
        // Register GML algebra functions
        let algebra = self.gml_algebra.clone();
        env.add_function("gml_state_function", move |l, c, n, alpha| {
            Ok(algebra.state_function(l, c, n, alpha))
        });
        
        // Register CNS adapter
        let cns = self.cns_adapter.clone();
        env.add_function("cns_start_span", move |op, attrs| {
            Ok(cns.start_span(op, attrs)?)
        });
        env.add_function("cns_end_span", move |status, attrs| {
            Ok(cns.end_span(status, attrs)?)
        });
        env.add_function("cns_record_error", move |code, msg| {
            Ok(cns.record_error(code, msg)?)
        });
        env.add_function("cns_check_variety", move |name, threshold| {
            Ok(cns.check_variety(name, threshold)?)
        });
        env.add_function("cns_audit_log", move |entry| {
            Ok(cns.audit_log(entry)?)
        });
    }
}
```

---

## Updated Templates

```jinja2
{# gml/recognize-ensemble.j2 (after) #}

{# CNS span start #}
{% do cns_start_span('recognize', {'concept_id': concept.id | default('unknown')}) %}

{# Validate inputs #}
{% set errors = validate_concept(concept) %}
{% if errors and errors | length > 0 %}
{% do cns_record_error('GML_INVALID_INPUT', errors | join('; ')) %}
{% include 'gml/error-validation.j2' %}
{% do cns_end_span('error', {'errors': errors | length}) %}
{% else %}

## Allosteric Recognition: {{ concept.name }}
...

{# Audit log #}
{% do cns_audit_log({
    'operation': 'recognize',
    'concept_id': concept.id,
    'capability_hash': capability.id.hash() | default('unknown'),
    'before_r_bar': None,
    'after_r_bar': r_bar,
    'result': 'success'
}) %}
{% do cns_end_span('success', {'r_bar': r_bar, 'n_h': n_h}) %}
{% endif %}
```

---

## CNS Emitter Implementation

```rust
/// Default CNS emitter (sends to hkask-cns)
pub struct DefaultCnsEmitter {
    cns_client: CnsClient,
}

impl CnsEmitter for DefaultCnsEmitter {
    fn emit(&self, event: CnsEvent) -> Result<(), CnsError> {
        self.cns_client.emit(event)
    }
    
    fn get_variety(&self, counter_name: &str) -> u64 {
        self.cns_client.get_variety(counter_name)
    }
}

/// Mock emitter for testing
pub struct MockCnsEmitter {
    events: Mutex<Vec<CnsEvent>>,
    variety_counters: Mutex<HashMap<String, u64>>,
}

impl MockCnsEmitter {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            variety_counters: Mutex::new(HashMap::new()),
        }
    }
    
    pub fn get_events(&self) -> Vec<CnsEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl CnsEmitter for MockCnsEmitter {
    fn emit(&self, event: CnsEvent) -> Result<(), CnsError> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
    
    fn get_variety(&self, counter_name: &str) -> u64 {
        *self.variety_counters.lock().unwrap().get(counter_name).unwrap_or(&0)
    }
}
```

---

## Testing

```rust
// hkask-testing/gml/cns_adapter_tests.rs

#[cfg(test)]
mod cns_adapter_tests {
    use super::*;
    
    #[test]
    fn test_start_end_span() {
        let emitter = MockCnsEmitter::new();
        let mut adapter = CnsAdapter::new(Box::new(emitter));
        
        let span_id = adapter.start_span("recognize", map!{"concept_id" => "gml:concept:freedom"}).unwrap();
        adapter.end_span(SpanStatus::Ok, map!{"r_bar" => 0.5}).unwrap();
        
        let events = emitter.get_events();
        assert_eq!(events.len(), 2);  // Start + End
        
        match &events[0] {
            CnsEvent::SpanStart { span, .. } => {
                assert_eq!(span, "cns.gml.recognize");
            }
            _ => panic!("Expected SpanStart"),
        }
    }
    
    #[test]
    fn test_audit_log() {
        let emitter = MockCnsEmitter::new();
        let mut adapter = CnsAdapter::new(Box::new(emitter));
        
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
        assert_eq!(events.len(), 1);
        
        match &events[0] {
            CnsEvent::AuditLog(entry) => {
                assert_eq!(entry.operation, "bind");
                assert_eq!(entry.before_r_bar, Some(0.1));
                assert_eq!(entry.after_r_bar, Some(0.6));
            }
            _ => panic!("Expected AuditLog"),
        }
    }
}
```

---

## Hexagonal Architecture Fit

```
┌─────────────────────────────────────────────────────────────┐
│                     Application Layer                        │
│                    (GML MCP Handlers)                        │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ uses
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   Inbound Ports                              │
│  ┌──────────────────────┐  ┌─────────────────────────────┐ │
│  │  GmlAlgebraService   │  │  CnsAdapter                 │ │
│  │  (domain logic)      │  │  (observability)            │ │
│  └──────────────────────┘  └─────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ implemented by
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                 Infrastructure Layer                         │
│  ┌──────────────────────┐  ┌─────────────────────────────┐ │
│  │  GmlAlgebraImpl      │  │  DefaultCnsEmitter          │ │
│  │  (MWC computations)  │  │  (sends to hkask-cns)       │ │
│  └──────────────────────┘  └─────────────────────────────┘ │
│                            ┌─────────────────────────────┐ │
│                            │  MockCnsEmitter             │ │
│                            │  (for testing)              │ │
│                            └─────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## Benefits

| Aspect | Before | After |
|--------|--------|-------|
| CNS coupling | Templates coupled | Adapter abstraction |
| Testability | Hard (Jinja2 macros) | Easy (mock emitter) |
| Type safety | Dynamic events | Typed events |
| Separation of concerns | Mixed | Clean boundary |
| Reusability | Template-specific | Reusable adapter |

---

*ℏKask — Planck's Constant of Agent Systems — GML v0.1.0*
*Task 8 complete: CNS adapter implementation specified.*
