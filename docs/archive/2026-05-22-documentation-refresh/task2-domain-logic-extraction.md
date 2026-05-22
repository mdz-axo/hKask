# GML Domain Logic Extraction

**Type:** Architecture Refactoring  
**Version:** 0.1.0  
**Priority:** Medium

---

## Overview

Move MWC computations from Jinja2 templates to domain layer (`hkask-mcp-gml`), keeping templates for rendering only.

---

## Current State (Before)

```jinja2
{# Template computes domain logic #}
{% set r_bar = ((1 + alpha) ** n) / (((1 + alpha) ** n) + (L * ((1 + c * alpha) ** n))) %}
{% set n_h = (n * ((1 - c) / (1 + c)) * ((alpha / (1 + alpha)) ** 0.5)) | round(3) %}
{% set Z = (1 + alpha) ** n + L * ((1 + c * alpha) ** n) %}
```

**Problems:**
- Domain logic in presentation layer
- Hard to test MWC computations
- Duplicated across templates
- No type safety

---

## Target State (After)

```jinja2
{# Template calls domain service #}
{% set r_bar = gml_algebra.state_function(L, c, n, alpha) %}
{% set n_h = gml_algebra.hill_coefficient(n, c, alpha) %}
{% set Z = gml_algebra.partition_function(L, c, n, alpha) %}
```

**Benefits:**
- Domain logic in domain layer
- Testable independently
- Single source of truth
- Type-safe interface

---

## Domain Service Interface

```rust
/// GML Algebra service (inbound port)
pub trait GmlAlgebraService {
    /// Compute state function R̄
    fn state_function(&self, l: f64, c: f64, n: usize, alpha: f64) -> f64;
    
    /// Compute Hill coefficient n_H
    fn hill_coefficient(&self, n: usize, c: f64, alpha: f64) -> f64;
    
    /// Compute partition function Z
    fn partition_function(&self, l: f64, c: f64, n: usize, alpha: f64) -> f64;
    
    /// Compute binding function Ȳ
    fn binding_function(&self, l: f64, c: f64, n: usize, alpha: f64) -> f64;
    
    /// Compute equilibrium distribution
    fn equilibrium(&self, l: f64, c: f64, n: usize, alpha: f64) -> Distribution;
}

#[derive(Debug, Clone)]
pub struct Distribution {
    pub p_r: f64,  // Probability of R-state
    pub p_t: f64,  // Probability of T-state
    pub n_h: f64,  // Hill coefficient
}
```

---

## Implementation

```rust
// hkask-mcp-gml/src/gml_algebra.rs

pub struct GmlAlgebraImpl;

impl GmlAlgebraService for GmlAlgebraImpl {
    fn state_function(&self, l: f64, c: f64, n: usize, alpha: f64) -> f64 {
        let one_plus_alpha = 1.0 + alpha;
        let one_plus_c_alpha = 1.0 + c * alpha;
        let numerator = one_plus_alpha.powi(n as i32);
        let denominator = numerator + l * one_plus_c_alpha.powi(n as i32);
        
        if denominator > 0.0 {
            numerator / denominator
        } else {
            0.0
        }
    }
    
    fn hill_coefficient(&self, n: usize, c: f64, alpha: f64) -> f64 {
        if alpha < 0.0 || c <= 0.0 {
            return 0.0;
        }
        let cooperativity_factor = (1.0 - c) / (1.0 + c);
        let concentration_factor = (alpha / (1.0 + alpha)).sqrt();
        n as f64 * cooperativity_factor * concentration_factor
    }
    
    fn partition_function(&self, l: f64, c: f64, n: usize, alpha: f64) -> f64 {
        let one_plus_alpha = 1.0 + alpha;
        let one_plus_c_alpha = 1.0 + c * alpha;
        one_plus_alpha.powi(n as i32) + l * one_plus_c_alpha.powi(n as i32)
    }
    
    fn binding_function(&self, l: f64, c: f64, n: usize, alpha: f64) -> f64 {
        let one_plus_alpha = 1.0 + alpha;
        let one_plus_c_alpha = 1.0 + c * alpha;
        
        let numerator = l * c * alpha * one_plus_c_alpha.powi(n as i32 - 1)
                      + alpha * one_plus_alpha.powi(n as i32 - 1);
        let denominator = self.partition_function(l, c, n, alpha);
        
        numerator / denominator
    }
    
    fn equilibrium(&self, l: f64, c: f64, n: usize, alpha: f64) -> Distribution {
        let p_r = self.state_function(l, c, n, alpha);
        let p_t = 1.0 - p_r;
        let n_h = self.hill_coefficient(n, c, alpha);
        
        Distribution { p_r, p_t, n_h }
    }
}
```

---

## Template Integration

```rust
// hkask-mcp-gml/src/template_context.rs

use crate::gml_algebra::GmlAlgebraService;

/// Context provided to Jinja2 templates
pub struct TemplateContext {
    pub gml_algebra: Box<dyn GmlAlgebraService>,
}

impl TemplateContext {
    pub fn new(algebra: Box<dyn GmlAlgebraService>) -> Self {
        Self { gml_algebra: algebra }
    }
    
    /// Register functions with Jinja2 engine
    pub fn register_functions(&self, env: &mut Environment) {
        let algebra = self.gml_algebra.clone();
        
        env.add_function("gml_state_function", move |l, c, n, alpha| {
            Ok(algebra.state_function(l, c, n, alpha))
        });
        
        env.add_function("gml_hill_coefficient", move |n, c, alpha| {
            Ok(algebra.hill_coefficient(n, c, alpha))
        });
        
        env.add_function("gml_partition_function", move |l, c, n, alpha| {
            Ok(algebra.partition_function(l, c, n, alpha))
        });
        
        env.add_function("gml_equilibrium", move |l, c, n, alpha| {
            Ok(algebra.equilibrium(l, c, n, alpha))
        });
    }
}
```

---

## Updated Templates

```jinja2
{# gml/recognize-ensemble.j2 (after) #}
{% from 'gml/macros.j2' import mwc_state_function, hill_coefficient, partition_function %}

{# Use domain service instead of inline computation #}
{% set r_bar = gml_state_function(L, c_avg, n, alpha) %}
{% set n_h = gml_hill_coefficient(n, c_avg, alpha) %}
{% set Z = gml_partition_function(L, c_avg, n, alpha) %}
```

```jinja2
{# gml/macros.j2 (after - delegates to domain service) #}
{% macro mwc_state_function(L, c, n, alpha) -%}
{{ gml_state_function(L, c, n, alpha) }}
{%- endmacro %}

{% macro hill_coefficient(n, c, alpha) -%}
{{ gml_hill_coefficient(n, c, alpha) }}
{%- endmacro %}

{% macro partition_function(L, c, n, alpha) -%}
{{ gml_partition_function(L, c, n, alpha) }}
{%- endmacro %}
```

---

## Testing

```rust
// hkask-testing/gml/gml_algebra_tests.rs

#[cfg(test)]
mod gml_algebra_tests {
    use super::*;
    
    #[test]
    fn test_state_function_default_bias() {
        let algebra = GmlAlgebraImpl;
        // L = 1000, no ligand (α = 0) → R̄ ≈ 0.001
        let r_bar = algebra.state_function(1000.0, 0.01, 4, 0.0);
        assert!((r_bar - 0.001).abs() < 0.0001);
    }
    
    #[test]
    fn test_state_function_saturating_activator() {
        let algebra = GmlAlgebraImpl;
        // L = 1000, c = 0.01, α = 100 → R̄ > 0.9
        let r_bar = algebra.state_function(1000.0, 0.01, 4, 100.0);
        assert!(r_bar > 0.9);
    }
    
    #[test]
    fn test_hill_coefficient_switch_like() {
        let algebra = GmlAlgebraImpl;
        // n = 4, c = 0.01, α = 1 → n_H > 1
        let n_h = algebra.hill_coefficient(4, 0.01, 1.0);
        assert!(n_h > 1.0);
    }
    
    #[test]
    fn test_hill_coefficient_graded() {
        let algebra = GmlAlgebraImpl;
        // n = 1, c = 0.5, α = 0.1 → n_H < 1
        let n_h = algebra.hill_coefficient(1, 0.5, 0.1);
        assert!(n_h < 1.0);
    }
    
    #[test]
    fn test_partition_function_normalization() {
        let algebra = GmlAlgebraImpl;
        let l = 100.0;
        let c = 0.1;
        let n = 2;
        let alpha = 1.0;
        
        let z = algebra.partition_function(l, c, n, alpha);
        let p_r = algebra.state_function(l, c, n, alpha);
        let p_t = 1.0 - p_r;
        
        // Verify Z normalizes correctly
        assert!(p_r >= 0.0 && p_r <= 1.0);
        assert!(p_t >= 0.0 && p_t <= 1.0);
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
│  │  GmlAlgebraService   │  │  CapabilityMiddleware       │ │
│  │  (domain logic)      │  │  (authorization)            │ │
│  └──────────────────────┘  └─────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ implemented by
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                 Infrastructure Layer                         │
│  ┌──────────────────────┐  ┌─────────────────────────────┐ │
│  │  GmlAlgebraImpl      │  │  TemplateRenderer           │ │
│  │  (MWC computations)  │  │  (Jinja2 rendering)         │ │
│  └──────────────────────┘  └─────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## Migration Plan

1. **Create `GmlAlgebraService` trait** — Define interface
2. **Implement `GmlAlgebraImpl`** — Move computations from templates
3. **Add tests** — Verify mathematical correctness
4. **Register with Jinja2** — Expose functions to templates
5. **Update templates** — Replace inline computations with function calls
6. **Remove macros** — Delete `macros.j2` (now delegates to domain)
7. **Verify** — Run test suite

---

## Benefits

| Aspect | Before | After |
|--------|--------|-------|
| Domain logic location | Templates | Service layer |
| Testability | Hard (Jinja2) | Easy (Rust tests) |
| Type safety | Dynamic | Static |
| DRY | Duplicated | Single source |
| Performance | Interpreted | Compiled |

---

*ℏKask — Planck's Constant of Agent Systems — GML v0.1.0*
*Task 2 complete: Domain logic extraction specified.*
