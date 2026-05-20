# Pragmatic Composition — Entity Relationship Diagram

## ℏKask v0.21.0

```mermaid
erDiagram
    SKILL ||--o{ TEMPLATE : "translates_to"
    SKILL ||--o{ MANIFEST : "translates_to"
    TEMPLATE ||--o{ LEXICON : "grounded_by"
    MANIFEST ||--|{ STEP : "defines"
    MANIFEST ||--o{ ENERGY_CAP : "enforces"
    TEMPLATE ||--o{ CONTRACT : "declares"
    REGISTRY ||--o{ TEMPLATE : "contains"
    REGISTRY ||--o{ MANIFEST : "contains"
    CNS ||--o{ SPAN : "emits"
    CNS ||--o{ VARIETY_COUNTER : "monitors"
    CNS ||--o{ ALGEDONIC_ALERT : "triggers"
    OCAP ||--o{ CAPABILITY : "grants"
    OCAP ||--o{ DELEGATION : "chains"
    CASCADE ||--o{ STAGE : "composes"
    CASCADE ||--o{ CASCADE : "references"
    
    SKILL {
        string id
        string name
        string source "Claude|Zapier|LangChain"
        json prompts
        json process_logic
        string visibility
    }
    
    TEMPLATE {
        string id
        string template_type "Prompt|Process|Cognition"
        string source_path
        array lexicon_terms
        object contract
        string frontmatter
        u64 energy_cap
        string visibility
    }
    
    MANIFEST {
        string id
        string name
        string description
        array steps
        u64 energy_cap
        string visibility
        string editor
    }
    
    STEP {
        int ordinal
        string action "select|populate|execute"
        string template_ref
        string model_tier
        string mcp
        string renderer
        object output_schema
    }
    
    CONTRACT {
        object input_schema
        object output_schema
        array required_fields
    }
    
    ENERGY_CAP {
        u64 max_budget
        u64 current_cost
        string unit "tokens/4"
        bool exceeded
    }
    
    LEXICON {
        string term
        string domain "WordAct|FlowDef|KnowAct"
        string definition
        bool canonical
    }
    
    REGISTRY {
        string root_path
        string index_method "filesystem|sqlite|git"
        bool cache_valid
    }
    
    CNS {
        string span_namespace "cns.*"
        u64 variety_counter
        u64 threshold
        bool algedonic_active
    }
    
    SPAN {
        string id
        string category "connector|pipeline|tool|prompt|agent_pod|energy"
        string phase "observe|regulate|outcome"
        json observation
        timestamp emitted_at
    }
    
    VARIETY_COUNTER {
        string entity_type
        u64 count
        u64 deficit
        bool alert_triggered
    }
    
    ALGEDONIC_ALERT {
        string id
        string severity "low|medium|high|critical"
        string reason
        timestamp triggered_at
        string escalated_to "Curator|Human"
    }
    
    OCAP {
        string owner_webid
        json capabilities
        json delegations
    }
    
    CAPABILITY {
        string id
        string resource
        string action
        string granted_by
        string granted_to
        object signature
        timestamp expires_at
    }
    
    DELEGATION {
        string id
        capability capability
        string delegator
        string delegate
        object signature
        string parent_id
    }
    
    CASCADE {
        string id
        array pre_stages
        array core_stages
        array post_stages
        u8 max_depth "7"
        SecurityAdapter security
    }
    
    STAGE {
        string name
        array templates
        string condition
    }
    
    CASCADE_CONTEXT {
        u8 current_depth
        HashSet visited_templates
        HashSet visited_manifests
        u64 energy_remaining
        Option CapabilityToken capability_token
        Vec u8 secret
        i64 current_time
    }
    
    SECURITY_ADAPTER {
        CapabilityChecker capability_checker
        HashSet allowed_paths
        Vec u8 secret
    }
    
    CAPABILITY_TOKEN {
        string id
        CapabilityResource resource
        string resource_id
        CapabilityAction action
        WebID delegated_from
        WebID delegated_to
        string signature
        Option i64 expires_at
        u8 attenuation_level
        u8 max_attenuation "7"
    }
```

## Cardinality Annotations

| Relationship | Cardinality | Description |
|-------------|-------------|-------------|
| SKILL → TEMPLATE | 1:N | One skill translates to multiple templates |
| SKILL → MANIFEST | 1:N | One skill translates to multiple manifests |
| TEMPLATE → LEXICON | N:M | Templates reference multiple lexicon terms |
| MANIFEST → STEP | 1:N | Manifest defines ordered step sequence |
| MANIFEST → ENERGY_CAP | 1:1 | Each manifest has one energy cap |
| TEMPLATE → CONTRACT | 1:1 | Each template has one input/output contract |
| REGISTRY → TEMPLATE | 1:N | Registry contains multiple templates |
| REGISTRY → MANIFEST | 1:N | Registry contains multiple manifests |
| CNS → SPAN | 1:N | CNS emits multiple span types |
| CNS → VARIETY_COUNTER | 1:N | CNS monitors multiple variety counters |
| CNS → ALGEDONIC_ALERT | 1:N | CNS triggers multiple alerts |
| OCAP → CAPABILITY | 1:N | OCAP grants multiple capabilities |
| OCAP → DELEGATION | 1:N | OCAP chains multiple delegations |
| CASCADE → STAGE | 1:N | Cascade composes multiple stages |
| CASCADE → CASCADE | N:N | Cascades can reference other cascades (recursive) |

## Security Boundaries (Bruce Schneier Threat Model)

```mermaid
graph TB
    subgraph "Trust Boundary 1: External Input"
        SKILL[External Skill]
        USER[User Input]
    end
    
    subgraph "Trust Boundary 2: Inbound Adapters"
        SIP[SkillImportPort]
        TCP[TemplateCompilePort]
        MVP[ManifestValidatePort]
    end
    
    subgraph "Trust Boundary 3: Core Domain (OCAP Enforced)"
        REG[Registry Core]
        CAS[Cascade Engine]
        CNS[CNS Monitor]
    end
    
    subgraph "Trust Boundary 4: Outbound Adapters"
        RWP[RegistryWritePort]
        CEP[CNSEmitPort]
        EAP[EnergyAccountPort]
    end
    
    subgraph "Trust Boundary 5: Persistent Storage"
        DB[(SQLite Registry)]
        CNS_DB[(CNS Event Log)]
    end
    
    SKILL -->|untrusted| SIP
    USER -->|untrusted| TCP
    SIP -->|validated| REG
    TCP -->|validated| REG
    MVP -->|validated| REG
    
    REG -->|capability-checked| RWP
    REG -->|capability-checked| CAS
    CAS -->|monitored| CNS
    CNS -->|alerts| EAP
    
    RWP -->|authenticated| DB
    CEP -->|authenticated| CNS_DB
    EAP -->|authenticated| CNS_DB
    
    style SKILL fill:#ff6b6b
    style USER fill:#ff6b6b
    style SIP fill:#feca57
    style TCP fill:#feca57
    style MVP fill:#feca57
    style REG fill:#48dbfb
    style CAS fill:#48dbfb
    style CNS fill:#48dbfb
    style RWP fill:#1dd1a1
    style CEP fill:#1dd1a1
    style EAP fill:#1dd1a1
    style DB fill:#5f27cd
    style CNS_DB fill:#5f27cd
```

### Threat Mitigations

| Threat | Mitigation | Implementation |
|--------|------------|----------------|
| **Path Traversal** | Input validation | `SecurityAdapter::validate_template_path()` blocks `..`, `/etc/`, absolute paths |
| **Template Injection** | Sandboxed Jinja2 | `minijinja` with restricted builtins |
| **Capability Forgery** | HMAC-SHA256 signatures | `CapabilityToken::sign()`, `CapabilityToken::verify()` |
| **Capability Escalation** | Attenuation on delegation | `CapabilityToken::attenuate()` increases level, reduces max |
| **Recursion Overflow** | Depth limiting | `MAX_CASCADE_DEPTH = 7`, `CascadeContext::check_depth()` |
| **Energy Exhaustion** | Budget caps | `CascadeContext::check_energy()`, `consume_energy()` |
| **Cycle Detection** | Visited set tracking | `CascadeContext::check_template_cycle()`, `check_manifest_cycle()` |
| **Unauthorized Access** | Capability validation | `CascadeContext::check_capability(resource, id, action)` |

### Security Integration Points

```rust
// CascadeExecutor integrates SecurityAdapter
pub struct CascadeExecutor {
    max_depth: u8,
    cycle_detection: bool,
    energy_tracking: bool,
    security: SecurityAdapter,  // Injected security
}

// Security checks on template resolution
for template_id in &stage.templates {
    self.security.validate_template_path(template_id)?;  // Path traversal check
    context.check_template_cycle(template_id)?;          // Cycle detection
    
    // Capability check if token present
    if context.capability_token.is_some() {
        context.check_capability(
            CapabilityResource::Template,
            &entry.id,
            CapabilityAction::Read,
        )?;
    }
}

// Capability attenuation on recursive calls
pub fn child_context(&self, new_holder: WebID) -> Self {
    let attenuated_token = self.capability_token.as_ref().and_then(|token| {
        if token.can_attenuate() {
            token.attenuate(new_holder, &self.secret, self.current_time)
        } else {
            None  // Max attenuation reached
        }
    });
    // ...
}
```
| **OCAP Bypass** | Runtime checks | `AccessEvaluator::evaluate()` |
| **Variety Deficit** | Algedonic alerts | `VarietyMonitor::check_threshold()` |
| **Data Exfiltration** | Visibility gating | `Visibility::Private|Shared|Public` |

*ℏKask v0.21.0 — Pragmatic Composition ERD*
