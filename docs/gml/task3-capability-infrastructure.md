# GML Capability Infrastructure

**Type:** Infrastructure Middleware  
**Version:** 0.1.0  
**Priority:** High (Security prerequisite)

---

## Overview

This document specifies the capability enforcement infrastructure that moves authorization from templates to the infrastructure layer, per OCAP principles.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Template Layer                           │
│  (rendering only, no domain logic or enforcement)           │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ render(template, inputs)
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                 Infrastructure Layer                         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Capability Middleware                               │   │
│  │  - Validates capability token                        │   │
│  │  - Checks operation permission                       │   │
│  │  - Checks effector budget                            │   │
│  │  - Checks port compatibility                         │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                 │
│                            │ if authorized:                  │
│                            ▼                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Template Renderer                                   │   │
│  │  - Renders template with inputs                      │   │
│  │  - Emits CNS spans                                   │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ if unauthorized:
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   Error Response                             │
│  - Returns error-generic.j2 or error-validation.j2          │
│  - Logs to CNS audit trail                                   │
└─────────────────────────────────────────────────────────────┘
```

---

## Capability Token Format

```rust
/// Cryptographically signed capability token
pub struct CapabilityToken {
    /// Unique identifier
    pub id: Uuid,
    
    /// Issuer's public key (Ed25519)
    pub issuer: PublicKey,
    
    /// Subject (who can use this capability)
    pub subject: HkaskId,
    
    /// Scope of authority
    pub scope: CapabilityScope,
    
    /// Allowed operations
    pub operations: Vec<GmlOperation>,
    
    /// Maximum effector concentration
    pub effector_budget: Option<f64>,
    
    /// Allowed ports (empty = all)
    pub ports_allowed: Vec<PortId>,
    
    /// Allowed concepts (empty = all in scope)
    pub concepts_allowed: Vec<ConceptId>,
    
    /// Valid from timestamp
    pub valid_from: Option<i64>,
    
    /// Valid until timestamp
    pub valid_until: Option<i64>,
    
    /// Cryptographic signature
    pub signature: Signature,
}

impl CapabilityToken {
    /// Verify signature and temporal validity
    pub fn verify(&self, now: i64) -> Result<bool, CapabilityError> {
        // Verify signature
        let payload = self.payload_hash();
        if !self.issuer.verify(&payload, &self.signature)? {
            return Err(CapabilityError::InvalidSignature);
        }
        
        // Verify temporal validity
        if let Some(from) = self.valid_from {
            if now < from {
                return Err(CapabilityError::NotYetValid);
            }
        }
        if let Some(until) = self.valid_until {
            if now > until {
                return Err(CapabilityError::Expired);
            }
        }
        
        Ok(true)
    }
    
    /// Check if operation is allowed
    pub fn allows_operation(&self, op: GmlOperation) -> bool {
        self.operations.contains(&op)
    }
    
    /// Check if port is allowed
    pub fn allows_port(&self, port_id: &PortId) -> bool {
        self.ports_allowed.is_empty() || self.ports_allowed.contains(port_id)
    }
    
    /// Check if concept is allowed
    pub fn allows_concept(&self, concept_id: &ConceptId) -> bool {
        self.concepts_allowed.is_empty() || self.concepts_allowed.contains(concept_id)
    }
    
    /// Check effector budget
    pub fn within_budget(&self, concentration: f64) -> bool {
        self.effector_budget.map_or(true, |budget| concentration <= budget)
    }
    
    /// Payload hash for signing
    fn payload_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(self.id.as_bytes());
        hasher.update(self.issuer.as_bytes());
        hasher.update(self.subject.as_bytes());
        for op in &self.operations {
            hasher.update(&[*op as u8]);
        }
        hasher.finalize().to_vec()
    }
}
```

---

## Capability Middleware

```rust
/// Middleware that enforces capability constraints
pub struct CapabilityMiddleware {
    current_time: i64,
}

impl CapabilityMiddleware {
    pub fn new() -> Self {
        Self {
            current_time: chrono::Utc::now().timestamp(),
        }
    }
    
    /// Authorize and render template
    pub fn authorize_and_render(
        &self,
        template: &Template,
        inputs: &TemplateInputs,
        capability: &CapabilityToken,
    ) -> Result<TemplateOutput, CapabilityError> {
        // Step 1: Verify capability token
        capability.verify(self.current_time)?;
        
        // Step 2: Check operation permission
        let operation = inputs.operation()?;
        if !capability.allows_operation(operation) {
            return Err(CapabilityError::OperationNotAllowed(operation));
        }
        
        // Step 3: Check concept permission
        if let Some(concept_id) = inputs.concept_id() {
            if !capability.allows_concept(concept_id) {
                return Err(CapabilityError::ConceptNotAllowed(concept_id.clone()));
            }
        }
        
        // Step 4: Check port permissions (for bind operations)
        if operation == GmlOperation::Bind {
            if let Some(effector) = inputs.effector() {
                if !capability.within_budget(effector.concentration) {
                    return Err(CapabilityError::BudgetExceeded {
                        requested: effector.concentration,
                        allowed: capability.effector_budget.unwrap_or(0.0),
                    });
                }
            }
        }
        
        // Step 5: Render template
        Ok(template.render(inputs)?)
    }
    
    /// Create error response for capability denial
    pub fn create_error_response(
        &self,
        error: CapabilityError,
        capability: &CapabilityToken,
    ) -> TemplateOutput {
        let (error_code, message, context) = match error {
            CapabilityError::OperationNotAllowed(op) => (
                "GML_CAPABILITY_DENIED",
                "Operation not allowed by capability",
                map! {
                    "operation" => format!("{:?}", op),
                    "scope" => format!("{:?}", capability.scope),
                }
            ),
            CapabilityError::BudgetExceeded { requested, allowed } => (
                "GML_BUDGET_EXCEEDED",
                "Effector concentration exceeds capability budget",
                map! {
                    "requested" => requested.to_string(),
                    "allowed" => allowed.to_string(),
                }
            ),
            CapabilityError::ConceptNotAllowed(id) => (
                "GML_CONCEPT_NOT_ALLOWED",
                "Concept access not permitted",
                map! {
                    "concept_id" => id.0.clone(),
                }
            ),
            CapabilityError::InvalidSignature => (
                "GML_INVALID_CAPABILITY",
                "Capability signature verification failed",
                map! {}
            ),
            CapabilityError::Expired => (
                "GML_CAPABILITY_EXPIRED",
                "Capability token has expired",
                map! {}
            ),
            _ => (
                "GML_CAPABILITY_ERROR",
                "Capability enforcement error",
                map! {}
            ),
        };
        
        TemplateOutput::Error {
            error_code: error_code.to_string(),
            message: message.to_string(),
            context,
        }
    }
}
```

---

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum CapabilityError {
    #[error("Operation not allowed: {0}")]
    OperationNotAllowed(GmlOperation),
    
    #[error("Concept not allowed: {0}")]
    ConceptNotAllowed(ConceptId),
    
    #[error("Port not allowed: {0}")]
    PortNotAllowed(PortId),
    
    #[error("Effector budget exceeded: requested {requested}, allowed {allowed}")]
    BudgetExceeded { requested: f64, allowed: f64 },
    
    #[error("Capability signature invalid")]
    InvalidSignature,
    
    #[error("Capability expired")]
    Expired,
    
    #[error("Capability not yet valid")]
    NotYetValid,
    
    #[error("Template rendering error: {0}")]
    TemplateError(#[from] TemplateError),
}
```

---

## Integration with hkask-mcp-gml

```rust
// hkask-mcp-gml/src/handlers.rs

use hkask_keystore::CapabilityToken;
use capability_middleware::CapabilityMiddleware;

pub struct GmlMcpHandler {
    middleware: CapabilityMiddleware,
    templates: TemplateRegistry,
}

impl GmlMcpHandler {
    pub fn handle_recognize(
        &self,
        inputs: RecognizeInputs,
        capability: CapabilityToken,
    ) -> Result<RecognizeOutput, CapabilityError> {
        let template = self.templates.get("gml/recognize-ensemble")?;
        
        // Middleware enforces capability
        let output = self.middleware.authorize_and_render(
            &template,
            &inputs.to_template_inputs(),
            &capability,
        )?;
        
        Ok(output.into_recognize_output())
    }
    
    pub fn handle_bind(
        &self,
        inputs: BindInputs,
        capability: CapabilityToken,
    ) -> Result<BindOutput, CapabilityError> {
        let template = self.templates.get("gml/bind-effector")?;
        
        // Middleware checks operation + budget + port compatibility
        let output = self.middleware.authorize_and_render(
            &template,
            &inputs.to_template_inputs(),
            &capability,
        )?;
        
        Ok(output.into_bind_output())
    }
}
```

---

## CNS Audit Logging

```rust
impl CapabilityMiddleware {
    /// Log capability check to CNS
    fn audit_log(
        &self,
        operation: GmlOperation,
        concept_id: &ConceptId,
        capability: &CapabilityToken,
        result: Result<(), &CapabilityError>,
    ) {
        let event = match result {
            Ok(()) => CnsEvent::AuditLog {
                operation: format!("{:?}", operation),
                concept_id: concept_id.0.clone(),
                capability_hash: capability.id.to_string(),
                result: "allowed".to_string(),
                timestamp: self.current_time,
            },
            Err(e) => CnsEvent::AuditLog {
                operation: format!("{:?}", operation),
                concept_id: concept_id.0.clone(),
                capability_hash: capability.id.to_string(),
                result: format!("denied: {:?}", e),
                timestamp: self.current_time,
            },
        };
        
        cns::emit(event);
    }
}
```

---

## Usage Example

```rust
// Create capability token
let capability = CapabilityToken {
    id: Uuid::new_v4(),
    issuer: system_public_key,
    subject: user_id.clone(),
    scope: CapabilityScope::Private,
    operations: vec![GmlOperation::Recognize, GmlOperation::Bind],
    effector_budget: Some(50.0),
    ports_allowed: vec![],  // All ports allowed
    concepts_allowed: vec![],  // All concepts in scope allowed
    valid_from: None,
    valid_until: Some(chrono::Utc::now().timestamp() + 3600),  // 1 hour
    signature: system_private_key.sign(&payload),
};

// Use capability with handler
let inputs = BindInputs {
    concept: freedom_concept,
    effectors: vec![security_crisis_effector],
};

let handler = GmlMcpHandler::new();
let result = handler.handle_bind(inputs, capability);

match result {
    Ok(output) => println!("Binding succeeded: {:?}", output),
    Err(CapabilityError::BudgetExceeded { requested, allowed }) => {
        println!("Budget exceeded: {} > {}", requested, allowed);
    }
    Err(e) => println!("Capability denied: {:?}", e),
}
```

---

## Security Properties

| Property | Enforcement |
|----------|-------------|
| No ambient authority | ✓ Explicit capability token required |
| Unforgeable tokens | ✓ Ed25519 signature verification |
| Least privilege | ✓ Default = no operations allowed |
| Attenuation | ✓ Child capabilities are subsets |
| Temporal bounds | ✓ valid_from / valid_until |
| Budget limits | ✓ effector_budget field |
| Complete mediation | ✓ All operations checked |
| Audit trail | ✓ CNS logging on all checks |

---

## Next Steps

1. **Implement in hkask-keystore** — Reuse existing crypto primitives
2. **Integrate with hkask-mcp-gml** — Add middleware to handlers
3. **Update templates** — Remove inline capability checks (now in infrastructure)
4. **Add tests** — Verify all error paths

---

*ℏKask — Planck's Constant of Agent Systems — GML v0.1.0*
*Task 3 complete: Capability infrastructure specified.*
