# Russell → hKask Migration — Implementation Status

**Date:** 2026-05-20  
**Status:** Phase 1 Complete ✅  
**Version:** v0.21.0

---

## Executive Summary

The Russell → hKask migration Phase 1 is **complete**. The migration infrastructure successfully transforms Russell skill manifests and prompt templates into hKask unified registry entries with full provenance tracking.

**Key Metrics:**
- **Rust LOC:** +574 lines (`russell_mapper.rs`)
- **Tests:** 4 unit tests (all passing)
- **Migrated Assets:** 4 Priority 1 items
- **Line Budget Impact:** +1.9% (well within 30,000 limit)

---

## Completed Work

### 1. Semantic Mapper Module ✅

**File:** `crates/hkask-templates/src/russell_mapper.rs` (574 LOC)

**Capabilities:**
- Russell skill manifest parsing (YAML)
- Russell prompt template parsing (Jinja2 with frontmatter extraction)
- Semantic transformation to hKask structures
- hLexicon term inference (keyword-based)
- Provenance tracking (SHA-256 hashes)
- Export formats: YAML, JSON, Mermaid ERD

**Key Structs:**
```rust
pub struct RussellSkillManifest {
    pub id: String,
    pub version: String,
    pub symptoms: Vec<String>,
    pub probes: Vec<RussellProbe>,
    pub interventions: Vec<RussellIntervention>,
    pub safety: RussellSafety,
}

pub struct RussellPromptTemplate {
    pub name: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub body: String,
    pub variables: Vec<String>,
}

pub struct MappedAsset {
    pub origin: String,
    pub asset_type: MappedAssetType,
    pub hkask_manifest: Option<ProcessManifest>,
    pub hkask_template: Option<CompositionTemplate>,
    pub lexicon_terms: Vec<String>,
    pub provenance_hash: String,
}
```

### 2. CLI Command ✅

**Command:** `kask registry import-russell`

**Options:**
```bash
kask registry import-russell --source <path> \
  --dry-run \
  --validate-only \
  --output-format yaml|json|mermaid \
  --verbose
```

**Implementation:** `crates/hkask-cli/src/commands.rs::import_russell()`

### 3. Priority 1 Asset Migration ✅

#### Skill Manifests (Process Type)

| Russell Origin | hKask Destination | Lexicon Terms | Steps | Status |
|----------------|-------------------|---------------|-------|--------|
| `russell/skills/web-search` | `registry/registries/skills/web-search.yaml` | search, fetch, browse, discover | 2 | ✅ |
| `russell/skills/pragmatic-semantics` | `registry/registries/skills/pragmatic-semantics.yaml` | observe, assess, classify, discriminate, validate | 1 | ✅ |
| `russell/skills/pragmatic-cybernetics` | `registry/registries/skills/pragmatic-cybernetics.yaml` | monitor, observe, assess, discriminate, validate | 1 | ✅ |

#### Prompt Templates

| Russell Origin | hKask Destination | Lexicon Terms | Variables | Status |
|----------------|-------------------|---------------|-----------|--------|
| `russell/.../soap.md.j2` | `registry/registries/prompt/soap.j2` | observe, assess, plan, act, monitor, recall, discover | 10 | ✅ |

### 4. Documentation ✅

| Document | Purpose | Location |
|----------|---------|----------|
| **Mapping ERD** | Mermaid ERD + transformation rules | `docs/architecture/russell-hkask-mapping-erd.md` |
| **Deferred Work** | Open questions + deferral rationale | `docs/architecture/registry-deferred-work.md` |
| **Open Questions** | 10 migration questions with defaults | `docs/architecture/OPEN_QUESTIONS.md` |
| **Future Work** | Completed work summary | `docs/architecture/future_work_resolved.md` |
| **Handoff** | Implementation status for agents | `docs/architecture/hKask-implementation-handoff.md` |

### 5. Unit Tests ✅

**Tests:** 4 passing
- `test_parse_russell_skill_manifest` — YAML parsing
- `test_transform_to_hkask_manifest` — Structure transformation
- `test_extract_jinja2_variables` — Variable extraction
- `test_infer_lexicon_terms` — hLexicon inference

**Command:** `cargo test -p hkask-templates russell_mapper::tests`

---

## Deferred Work

### Deferred Items (5 total)

| ID | Item | Priority | Decision | Trigger for Revisit |
|----|------|----------|----------|---------------------|
| **D1** | CNS integration for migration spans | Medium | Defer until operational data | 10+ successful migrations |
| **D2** | OCAP capability enforcement | Medium | Defer until security audit | Security audit completion |
| **D3** | Bidirectional sync strategy | Low | One-time migration | Russell upstream breaking changes |
| **D4** | hLexicon term inference | Low | Manual specification | Migration scale bottleneck |
| **D5** | Cascade composition wrapping | Low | Flat templates | Cascade utility data |

**Full Details:** See `docs/architecture/registry-deferred-work.md#russell-migration-deferred-work-2026-05-20`

### Open Questions (10 total)

| # | Question | Default Decision |
|---|----------|------------------|
| **RQ1** | Bidirectional sync with Russell upstream? | One-time migration |
| **RQ2** | hLexicon term inference (LLM vs. manual)? | Manual specification |
| **RQ3** | Capability granularity (coarse vs. fine)? | Coarse (preserve Russell) |
| **RQ4** | Provenance log retention policy? | Indefinite |
| **RQ5** | Template versioning (Russell vs. Git)? | Git SHA (Russell as metadata) |
| **RQ6** | Cascade composition wrapping? | Flat templates |
| **RQ7** | Bot vs. Replicant mapping criteria? | Bot mapping |
| **RQ8** | MCP tool discovery (inference vs. manual)? | Manual specification |
| **RQ9** | Error recovery (rollback vs. skip)? | Skip-on-error |
| **RQ10** | Registry bloat prevention? | Migrate-all with utility tags |

**Full Details:** See `docs/architecture/OPEN_QUESTIONS.md#russell-migration-open-questions-2026-05-20`

---

## Verification

### Compilation

```bash
cargo check -p hkask-templates
# Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
```

### Tests

```bash
cargo test -p hkask-templates russell_mapper::tests
# running 4 tests
# test russell_mapper::tests::test_infer_lexicon_terms ... ok
# test russell_mapper::tests::test_transform_to_hkask_manifest ... ok
# test russell_mapper::tests::test_parse_russell_skill_manifest ... ok
# test russell_mapper::tests::test_extract_jinja2_variables ... ok
#
# test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Line Budget

```bash
wc -l crates/hkask-templates/src/russell_mapper.rs
# 574 crates/hkask-templates/src/russell_mapper.rs

# Total workspace Rust LOC: ~5,709 (19% of 30,000 budget)
```

### Migrated Assets

```bash
ls -lh registry/registries/skills/ registry/registries/prompt/
# registry/registries/prompt/:
# total 16K
# -rw-rw-r-- 1 mdz-axolotl mdz-axolotl 2.4K May 20 08:26 soap.j2
#
# registry/registries/skills/:
# total 12K
# -rw-rw-r-- 1 mdz-axolotl mdz-axolotl 1.5K May 20 08:25 pragmatic-cybernetics.yaml
# -rw-rw-r-- 1 mdz-axolotl mdz-axolotl 1.4K May 20 08:25 pragmatic-semantics.yaml
# -rw-rw-r-- 1 mdz-axolotl mdz-axolotl 1.7K May 20 08:25 web-search.yaml
```

---

## Architecture Compliance

### Hexagonal Architecture ✅

- **Inbound Ports (Driven):** `RussellMapper::analyze_*()`, CLI command
- **Core Domain (Rust):** Semantic mapper, transformation logic, provenance tracking
- **Outbound Ports (Driving):** Registry writer (deferred), CNS adapter (deferred)
- **Soft Layer (YAML/Jinja2):** Migrated manifests and templates

### Code vs. Content Separation ✅

| Layer | Technology | Budget | Status |
|-------|------------|--------|--------|
| Hard (Kernel) | Rust | ≤30,000 LOC | 5,709 LOC (19%) |
| Soft (Material) | YAML, Jinja2 | Unlimited | 6.5 KB migrated |
| Testing | Rust (tests) | Unlimited | 4 tests |

### Five Anchors Alignment ✅

| Anchor | Implementation | Status |
|--------|----------------|--------|
| Agent Enablement | Bots execute migrated manifests | ✅ |
| Essential Tools | 10 MCP servers referenced | ✅ |
| User Sovereignty | OCAP, provenance tracking | ✅ (deferred enforcement) |
| CNS | Migration spans documented | ✅ (deferred integration) |
| Composition | Unified registry with template_type | ✅ |

---

## Next Steps

### Immediate (This Week)

1. **Priority 2 Migration** — Migrate remaining high-utility Russell assets:
   - `russell/skills/scenario-tester/` — Complex, requires CNS integration
   - `russell/skills/skill-manager/` — Depends on registry completion

2. **Operational Data Collection** — Run migrations with `--verbose` to gather:
   - hLexicon inference accuracy
   - Transformation success/failure rates
   - Provenance hash verification results

### Short Term (This Month)

3. **CNS Integration** — Activate migration spans based on operational data
4. **OCAP Enforcement** — Implement capability token validation for migrated manifests
5. **Security Audit** — Review migrated assets for security boundary compliance

### Long Term (Next Quarter)

6. **Bidirectional Sync** — Implement if Russell upstream evolves
7. **LLM-based hLexicon Inference** — If manual specification becomes bottleneck
8. **Cascade Compositions** — Wrap flat templates if patterns emerge

---

## References

- **Architecture Spec:** `docs/architecture/hKask-architecture-master.md`
- **Registry Design:** `docs/architecture/registry-templating-prompt-v2.md`
- **Mapping ERD:** `docs/architecture/russell-hkask-mapping-erd.md`
- **Deferred Work:** `docs/architecture/registry-deferred-work.md`
- **Open Questions:** `docs/architecture/OPEN_QUESTIONS.md`
- **Implementation Handoff:** `docs/architecture/hKask-implementation-handoff.md`

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*Rust is the loom. YAML/Jinja2 is the thread. Russell is the legacy library. hKask is the unified registry.*
