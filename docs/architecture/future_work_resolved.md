# ℏKask Registry & Templating — Open Questions Resolved

**Generated:** 2026-05-20  
**Status:** 7/10 questions resolved  
**Deferred:** 3 questions (require operational data)

---

## Question 1: Lexicon Binding Intelligence

**Question:** Should lexicon term selection be LLM-driven (semantic similarity) or rule-based (exact match)?

### Optional Solutions

| Solution | Description | Pros | Cons |
|----------|-------------|------|------|
| **A: LLM-driven** | Use fast local model to compute semantic similarity between input context and lexicon terms | Flexible, handles synonyms and related concepts | Costs energy (200-500 tokens per lookup), may drift over time |
| **B: Rule-based** | Exact string match or prefix match against lexicon term list | Deterministic, zero energy cost, fast | Brittle, no tolerance for variation |
| **C: Hybrid** | Rule-based first, LLM fallback on no match | Best of both: fast path + flexible fallback | Complexity in implementation |

### Recommended Answer: **C: Hybrid**

**Rationale:**
- hKask design principle P4 (No builder without complexity): The hybrid approach justifies complexity because lexicon binding is a core primitive
- Energy budget: 90% of bindings will be exact matches (zero cost), 10% fallback to LLM (bounded cost)
- CNS tracking: Emit `cns.lexicon.match` span with `match_type: exact|semantic` for calibration

**Implementation:**
```rust
fn bind_lexicon_term(input: &str, lexicon: &Lexicon) -> Option<LexiconTerm> {
    // Fast path: exact match
    if let term = lexicon.get_exact(input) {
        emit_cns_span("cns.lexicon.match", &{"match_type": "exact"});
        return Some(term);
    }
    
    // Fallback: semantic similarity via LLM
    if let term = lexicon.get_semantic(input, inference_port) {
        emit_cns_span("cns.lexicon.match", &{"match_type": "semantic"});
        return Some(term);
    }
    
    None
}
```

**Status:** ✅ RESolved — Implement hybrid with CNS tracking for calibration.

---

## Question 2: Recursive Template Composition

**Question:** Can templates reference other templates (e.g., `{{ render_template("base_prompt", vars) }}`)?

### Optional Solutions

| Solution | Description | Pros | Cons |
|----------|-------------|------|------|
| **A: No recursion** | Templates cannot reference other templates | Simple, no cycle risk, easy to audit | Loses composability |
| **B: Depth-limited recursion** | Allow recursion up to 3 levels with cycle detection | Enables composition, bounded energy | Implementation complexity |
| **C: Manifest-only composition** | Composition via manifest steps, not template references | Clear separation, manifest owns flow | Less flexible for template authors |

### Recommended Answer: **B: Depth-limited recursion**

**Rationale:**
- Miller's law: 7±2 cognitive load, but 3 is safer for recursion (matroshka limit)
- hKask constraint C7 (When implementations diverge, one must yield): Yield to energy budget by imposing hard limit
- CNS algedonic alert: Emit `cns.recursion.depth` span; alert if depth > 3

**Implementation:**
```rust
const MAX_MATROSHKA_DEPTH: u8 = 3;

fn render_template(template: &Template, vars: Value, depth: u8) -> Result<String> {
    if depth > MAX_MATROSHKA_DEPTH {
        emit_algedonic_alert("recursion_overflow", depth);
        return Err(Error::RecursionLimit);
    }
    
    // Detect cycle via call stack tracking
    if call_stack.contains(template.id) {
        return Err(Error::CycleDetected);
    }
    
    // Render with nested template support
    let rendered = minijinja_render(template.source, vars, depth + 1)?;
    emit_cns_span("cns.recursion.depth", &{"depth": depth});
    Ok(rendered)
}
```

**Status:** ✅ Resolved — Allow recursion with depth limit 3 and cycle detection.

---

## Question 3: Dynamic Energy Cap Calibration

**Question:** Should energy caps be static (declared in manifest) or dynamic (learned from historical usage)?

### Optional Solutions

| Solution | Description | Pros | Cons |
|----------|-------------|------|------|
| **A: Static caps** | Fixed values in manifest YAML | Predictable, auditable, simple | May be over/under-allocated |
| **B: Dynamic caps** | Learned from CNS historical outcomes | Optimizes actual usage, adaptive | Requires feedback loop, complexity |
| **C: Static with manual override** | Static caps + curator can adjust via CLI/API | Simple + flexible | Human in loop required |

### Recommended Answer: **A: Static caps initially, B: Dynamic after baseline**

**Rationale:**
- hKask design principle P6 (Delete stubs, don't publish them): Dynamic calibration requires CNS baseline data; don't implement until data exists
- Start with static caps (4096, 8192, etc.) based on token cost estimation
- After 100+ executions, CNS can recommend cap adjustments via `cns.energy.calibrate` span

**Implementation Phase 1 (Static):**
```yaml
energy:
  cap: 8192  # Fixed value
  hard_limit: true
```

**Implementation Phase 2 (Dynamic, deferred):**
```yaml
energy:
  cap: auto  # Learned from CNS history
  learning_window: 100  # executions
  variance_threshold: 0.2
```

**Status:** ✅ Resolved — Start static; add dynamic calibration after CNS baseline is collected (≥100 executions).

---

## Question 4: Multi-Tenant Registry Isolation

**Question:** Should registry support tenant-scoped templates (Private visibility per-user)?

### Optional Solutions

| Solution | Description | Pros | Cons |
|----------|-------------|------|------|
| **A: Single-tenant** | All templates Shared/Public; no user scoping | Simple, no isolation complexity | No private templates |
| **B: SQLCipher + OCAP** | Encrypt per-user, capability-enforced access | Full isolation, secure | Complexity, SQLCipher integration required |
| **C: Filesystem scoping** | Separate directories per user | Simpler than SQLCipher | Weaker isolation |

### Recommended Answer: **A: Single-tenant for MVP**

**Rationale:**
- hKask constraint C1 (A type must be worn before it's tailored): User model not yet defined; cannot tailor isolation before user exists
- Defer to post-MVP when user authentication is implemented
- Visibility types (`Private|Shared|Public`) can be added later without breaking changes

**Status:** ⏸️ Deferred — Implement single-tenant first; add multi-tenant after user model is defined.

---

## Question 5: Template Versioning and Migration

**Question:** How are breaking changes to templates handled (e.g., contract field changes)?

### Optional Solutions

| Solution | Description | Pros | Cons |
|----------|-------------|------|------|
| **A: Semantic versioning** | SemVer on templates (v1.0.0, v2.0.0) | Explicit breaking changes | Complexity in tracking |
| **B: Git-only versioning** | Git SHA tracks changes; no SemVer | Simpler, aligns with hKask design | Less explicit for breaking changes |
| **C: Curator migration tool** | Curator manually migrates templates on contract change | Human oversight | Manual effort required |

### Recommended Answer: **B: Git-only versioning**

**Rationale:**
- AGENTS.md explicitly states: "SemVer versioning (Git-only)" as a hallucination to avoid
- Git SHA provides sufficient provenance tracking
- CNS `cns.prompt.outcome` span tracks contract shape; divergence detected automatically

**Implementation:**
```rust
// Template metadata includes Git SHA
struct Template {
    id: String,
    source_path: String,
    git_sha: String,  // Provenance
    contract_shape: ContractShape,  // For divergence detection
}

// CNS detects contract shape changes
if new_template.contract_shape != old_template.contract_shape {
    emit_cns_span("cns.prompt.contract_drift", &{"git_sha": new_template.git_sha});
}
```

**Status:** ✅ Resolved — Git-only versioning; CNS detects contract drift.

---

## Question 6: MCP Tool Template Caching

**Question:** Should rendered templates be cached to reduce energy cost on repeated invocations?

### Optional Solutions

| Solution | Description | Pros | Cons |
|----------|-------------|------|------|
| **A: No caching** | Always re-render | Fresh, no staleness risk | Wastes energy on repeated calls |
| **B: TTL cache** | Cache rendered output for 5 minutes | Reduces energy, bounded staleness | Cache invalidation complexity |
| **C: Input-hash cache** | Cache keyed by (template_id, input_hash) | Precise, no staleness | Memory growth over time |

### Recommended Answer: **B: TTL cache with 5-minute expiry**

**Rationale:**
- Energy budget: Rendering same template twice in 5 minutes is likely redundant
- TTL bound prevents staleness beyond 5 minutes
- CNS can emit `cns.cache.hit` / `cns.cache_miss` spans for calibration

**Implementation:**
```rust
struct RenderCache {
    entries: HashMap<String, CachedEntry>,
    ttl_seconds: u64 = 300,  // 5 minutes
}

fn get_cached(template_id: &str, input_hash: &str) -> Option<String> {
    let key = format("{}:{}", template_id, input_hash);
    cache.entries.get(&key).and_then(|e| {
        if e.expires_at > now() {
            emit_cns_span("cns.cache.hit", &{"key": key});
            Some(e.rendered)
        } else {
            None  // TTL expired
        }
    })
}
```

**Status:** ✅ Resolved — TTL cache (5 minutes) with CNS tracking for calibration.

---

## Question 7: Cross-Registry Federation

**Question:** Should hKask registry support federation with external registries (e.g., MCP server registries)?

### Optional Solutions

| Solution | Description | Pros | Cons |
|----------|-------------|------|------|
| **A: No federation** | Local registry only | Simple, trusted, auditable | No external templates |
| **B: Federation with trust model** | OCAP capabilities across registries | Extensible, interoperable | Trust boundary complexity |
| **C: Import-only** | Manual import from external registries | Controlled, audited import | No runtime federation |

### Recommended Answer: **A: No federation for MVP**

**Rationale:**
- Bruce Schneier: "Security is a process, not a product." Federation introduces trust boundaries before security process is validated
- Mark Miller: "No ambient authority." Federation requires cross-regist authority delegation
- Defer until OCAP trust model is validated in single-registry context

**Status:** ⏸️ Deferred — Implement local registry first; add federation after OCAP validation.

---

## Question 8: hLexicon Validation Timing

**Question:** Load-time or render-time lexicon validation? Failure mode if unknown term referenced?

### Optional Solutions

| Solution | Description | Pros | Cons |
|----------|-------------|------|------|
| **A: Load-time** | Validate lexicon terms when template loads | Fast failure, early detection | Requires lexicon to be loaded first |
| **B: Render-time** | Validate when template renders | Flexible, lexicon can evolve | Late failure, harder to debug |
| **C: Both** | Load-time check + render-time recheck | Best detection | Double validation cost |

### Recommended Answer: **C: Both**

**Rationale:**
- C5 (Every error variant is a unique recovery path): Load-time and render-time failures have different recovery strategies
- Load-time: Fail template registration ( curator fixes)
- Render-time: Fail execution with CNS span (system adapts)

**Implementation:**
```rust
// Load-time validation
fn register_template(template: Template) -> Result<()> {
    for term in &template.lexicon_terms {
        if !lexicon.contains(term) {
            return Err(Error::LexiconTermMissing(term.clone()));
        }
    }
    Ok(())
}

// Render-time recheck
fn render_template(template: &Template, vars: Value) -> Result<String> {
    for term in &template.lexicon_terms {
        if !lexicon.contains(term) {
            emit_cns_span("cns.lexicon.drift", &{"term": term});
            return Err(Error::LexiconDrift(term.clone()));
        }
    }
    // ... render
}
```

**Status:** ✅ Resolved — Validate at load-time (registration) and render-time (execution).

---

## Question 9: Cross-Registry Composition

**Question:** Can Process template invoke Prompt template? Can Cognition invoke Process? What are composition rules?

### Optional Solutions

| Solution | Description | Pros | Cons |
|----------|-------------|------|------|
| **A: No cross-type invocation** | Each type isolated | Simple, clear boundaries | Loses composability |
| **B: Typed composition** | Process → Prompt OK; Cognition → Process OK; reverse forbidden | Enables composition, prevents cycles | Complexity in type checking |
| **C: Free composition** | Any type can invoke any other | Most flexible | Risk of cycles, hard to audit |

### Recommended Answer: **B: Typed composition**

**Rationale:**
- hKask template types form a dependency hierarchy: `Cognition → Process → Prompt`
- This prevents cycles (Cognition cannot invoke itself)
- Aligns with hexagonal architecture: Cognition (think) → Process (do) → Prompt (say)

**Composition Rules:**
```
Cognition → Process → Prompt  (allowed)
Cognition → Prompt            (allowed)
Process → Prompt              (allowed)
Process → Cognition           ( forbidden: cycle risk)
Prompt → Process              ( forbidden: prompt is leaf)
Prompt → Cognition            ( forbidden: prompt is leaf)
```

**Status:** ✅ Resolved — Typed composition with hierarchy: `Cognition → Process → Prompt`.

---

## Question 10: Bootstrap Loading Order

**Question:** Are dispatch manifest and selector template loaded by convention from fixed paths, or is there a Rust bootstrap sequence?

### Optional Solutions

| Solution | Description | Pros | Cons |
|----------|-------------|------|------|
| **A: Convention-based** | Fixed paths (`registry/manifests/dispatch.yaml`, `registry/templates/selector.jinja2`) | Simple, no bootstrap logic | Less flexible |
| **B: Rust bootstrap** | Rust loads bootstrap config first, then resolves paths | Flexible, configurable | Complexity in bootstrap |
| **C: Hybrid** | Convention with override config | Best of both | Slight complexity |

### Recommended Answer: **A: Convention-based for MVP**

**Rationale:**
- P6 (Delete stubs, don't publish them): Bootstrap logic is stub until proven necessary
- Convention paths are inspectable and auditable
- Override can be added later via config field if needed

**Convention Paths:**
```
registry/manifests/dispatch.yaml       — Bootstrap manifest
registry/templates/selector.jinja2     — Bootstrap template
registry/bots/registry-dispatch-bot.yaml — Bot manifest
```

**Status:** ✅ Resolved — Convention-based loading from fixed paths.

---

## Summary Table

| # | Question | Resolution | Implementation Phase |
|---|----------|------------|----------------------|
| 1 | Lexicon Binding | Hybrid (exact + semantic fallback) | Phase 1 |
| 2 | Recursive Composition | Depth-limited (max 3) | Phase 1 |
| 3 | Energy Cap Calibration | Static first, dynamic later | Phase 1 (static), Phase 2 (dynamic) |
| 4 | Multi-Tenant Isolation | Deferred | Post-MVP |
| 5 | Template Versioning | Git-only | Phase 1 |
| 6 | MCP Tool Caching | TTL cache (5 min) | Phase 1 |
| 7 | Cross-Registry Federation | Deferred | Post-MVP |
| 8 | hLexicon Validation | Load-time + render-time | Phase 1 |
| 9 | Cross-Registry Composition | Typed hierarchy | Phase 1 |
| 10 | Bootstrap Loading | Convention-based | Phase 1 |

---

*ℏKask v0.21.0 — Planck's Constant of Agent Systems*
*As simple as possible, but no simpler.*