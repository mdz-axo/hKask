# Nika → hKask Skill System Cross-Pollination Audit — Follow-Up

**Date:** 2026-07-21 · **Mode:** advisory (read-only analysis + task plan) · **Author:** agent (Zed)
**Parent document:** `tasks/nika-skill-audit.md`
**Directives addressed:**
1. Lexicon rationalization (the lexicon was meant to be the vocabulary; it has become something else)
2. CNS namespace reorganization by domain/subdomain (defend against creep; `cns.skill.*` as a domain with subdomains)
3. Proceed as recommended (R3 + R6 trait coordination)
4. Follow-up audit reading Nika's verb crates (`infer`/`exec`/`invoke`/`agent`) in source

> **Honesty note:** Nika's four verb crates (`nika-verb-infer`, `nika-verb-exec`, `nika-verb-invoke`, `nika-verb-agent`) were read in full from `main` (v0.105.0). hKask's `CANONICAL_NAMESPACES` array was read in full from `crates/hkask-types/src/event.rs` L111-323. The lexicon situation was verified by grep — no `Lexicon` type exists in `hkask-types`; `lexicon_terms` is a free-form `Vec<String>` on `RegistryEntry` and `BundleSkill`.

---

## Directive 1 — Lexicon Rationalization

### 1.1 Current state (verified)

The "lexicon" in hKask is **not a controlled vocabulary**. It is a free-form tag list:

- `crates/hkask-ports/src/registry.rs` L9-19: `RegistryEntry { lexicon_terms: Vec<String>, ... }` — a `Vec<String>`, no enum, no validation, no closed set.
- `crates/hkask-templates/src/bundle/manifest.rs` L22-29: `BundleSkill { lexicon_terms: Vec<String>, ... }` — same shape.
- `crates/hkask-services-skill/src/audit.rs` L1007-1015: `J2FrontMatter { lexicon_terms: Vec<String>, ... }` — same shape, parsed from Jinja2 frontmatter.
- `crates/hkask-api/src/routes/templates.rs` L40-44: `TemplateResponse { lexicon_terms: Vec<String>, ... }` — exposed in the HTTP API.
- `crates/hkask-api/src/routes/templates.rs` L127: `search_templates` calls `registry.search_by_lexicon(&term)` — the lexicon is a *search index*, not a vocabulary.

**No `Lexicon` type, no `LexiconTerm` enum, no vocabulary file, no validation against a closed set.** The grep for `Lexicon|lexicon_term|LexiconTerm` in `hkask-types/**/*.rs` returned zero matches.

### 1.2 What it was meant to be (inferred from the doc comments)

The doc comments say things like "Canonical vocabulary terms this template implements" (`templates.rs` L28) and "Canonical vocabulary terms" (`registry.rs` L13). The word "canonical" implies a closed set — but no closed set exists. The lexicon has drifted from "controlled vocabulary" to "free-form tag list used as a search index."

### 1.3 The rationalization question

You asked whether to "streamline, consolidate, or rationalize" the lexicon. Three options, ranked by essentialist G1 (deletion test):

#### Option A — Delete the lexicon entirely
- **G1:** Inline into callers — `search_by_lexicon` would need a replacement (full-text search over `name` + `description`). Complexity reappears (search quality drops; the lexicon was a curated tag index). Delete the lexicon — `search_templates` loses its primary access path. → **FAIL G1** (behavior lost).

#### Option B — Keep the free-form `Vec<String>`, add validation against a closed set
- **G1:** Inline into callers — callers currently pass any string. If we delete the closed set, callers go back to free-form — complexity reappears (typos like `classfy` silently index as a new term). Delete the closed set — typo-detection impossible. → **PASS G1.**
- **G2:** Adds 1 enum (`LexiconTerm`) + 1 validation function. The enum is the *vocabulary*; the validation is the *gate*. Depth score: high (the vocabulary encodes the domain). → **PASS G2.**
- **G3:** Genuine behavior (typo rejection + vocabulary enforcement). Not a pass-through. → **PASS G3.**
- **Cost:** Define the closed set. This is the hard part — what are the canonical terms? Looking at the test fixtures (`audit.rs` L1157, L1220, L1285), the only term actually used in tests is `classify`. The manifests in `registry/manifests/` use `functional_role: flowdef` and `category: skill` — these are *already* closed-set fields. The `lexicon_terms` field may be redundant with `functional_role` + `category` + `template_type`.

#### Option C — Replace `lexicon_terms` with the existing closed-set fields (`functional_role`, `category`, `template_type`)
- **G1:** Inline into callers — `search_by_lexicon` would search over `functional_role` + `category` + `template_type` instead. These are already typed enums. Complexity *does not* reappear (the search index becomes the typed enums, which is *better* than free-form strings). Delete `lexicon_terms` — search still works via the typed enums. → **PASS G1** (behavior preserved via the typed enums).
- **G2:** Removes 1 field (`lexicon_terms`) from 4 structs. Adds 0 new public items. Interface shrinks. → **PASS G2.**
- **G3:** The typed enums (`TemplateType`, `functional_role`, `category`) already encode the vocabulary. `lexicon_terms` is a *pass-through* that duplicates them as strings. → **FAIL G3** (pass-through abstraction — the essentialist G3 test catches this).

**Verdict:** Option C is the essentialist winner *if* the typed enums cover the search use case. But the test fixtures use `lexicon_terms: [classify]` — `classify` is not a `TemplateType` (FlowDef/KnowAct/WordAct/RenderAct) or a `category` (skill/qa-script/runtime-config/...). So `lexicon_terms` carries *domain semantics* that the typed enums don't.

**Recommendation (advisory):** **Option B, but with the closed set derived from the existing manifests.** Audit `registry/manifests/*.yaml` for all `lexicon_terms` values actually in use; promote that set to a `LexiconTerm` enum; validate at parse time. If the set is small (≤20), this is a deep module. If the set is large or open, Option A (delete + full-text search) is better — but that's a bigger change.

**Open question for you:** Is the lexicon meant to be (a) a closed vocabulary of domain verbs (classify, diagnose, audit, forecast, ...), or (b) an open tag list for search? If (a), Option B. If (b), delete the "canonical" language from the doc comments and admit it's a tag list.

---

## Directive 2 — CNS Namespace Reorganization

### 2.1 Current state (verified)

`crates/hkask-types/src/event.rs` L111-323: `CANONICAL_NAMESPACES` is a flat `&[&str]` array with **~110 entries**. The hierarchical validation rule (`is_canonical`, L328-338) accepts any descendant of a registered namespace:

```rust
fn is_canonical(namespace: &str) -> bool {
    if CANONICAL_NAMESPACES.contains(&namespace) { return true; }
    if let Some(last_dot) = namespace.rfind('.') {
        is_canonical(&namespace[..last_dot])
    } else { false }
}
```

This means **registering `cns.skill` (L239) silently accepts `cns.skill.cascade`, `cns.skill.converged`, `cns.skill.escalated`, `cns.skill.gas_exhausted`, `cns.skill.gas_alert`, `cns.skill.rjoule_exhausted`, `cns.skill.rjoule_alert`, `cns.skill.compute`, `cns.skill.frontmatter_missing`, `cns.skill.manifest_unparseable`, `cns.skill.manifest_absent`, `cns.skill.skill_activated`, `cns.skill.skills_loaded`, `cns.skill.skills_discovered`, `cns.skill.skill_published`, `cns.skill.registry_validated`** — and *any other* `cns.skill.*` string anyone writes in a `tracing::target`.

### 2.2 The creep problem

The grep for `cns.skill\.` found **18 distinct sub-namespaces** in use in `executor.rs` alone, plus more in `skill_impl.rs`. None of them are registered in `CANONICAL_NAMESPACES`. They all pass `is_canonical` only because `cns.skill` is registered and the hierarchical rule accepts any descendant.

This is exactly the "namespace creep" you flagged. The hierarchical rule is *too permissive* — it was designed to allow deep sub-namespaces without registration, but the side effect is that any typo or ad-hoc sub-namespace silently validates.

### 2.3 The reorganization proposal

You asked to "organize the CNS namespace by domain and subdomain" with `cns.skills.*` as a domain. Two changes:

#### Change 1 — Register subdomains explicitly (defend against creep)

Replace the hierarchical `is_canonical` rule with an **exact-match** rule, and register every sub-namespace explicitly. This forces every new sub-namespace to be added to `CANONICAL_NAMESPACES`, making creep visible in code review.

- **Current:** `cns.skill` registered → any `cns.skill.*` accepted.
- **Proposed:** `cns.skill.cascade`, `cns.skill.converged`, `cns.skill.escalated`, `cns.skill.gas_exhausted`, `cns.skill.gas_alert`, `cns.skill.rjoule_exhausted`, `cns.skill.rjoule_alert`, `cns.skill.compute`, `cns.skill.frontmatter_missing`, `cns.skill.manifest_unparseable`, `cns.skill.manifest_absent`, `cns.skill.skill_activated`, `cns.skill.skills_loaded`, `cns.skill.skills_discovered`, `cns.skill.skill_published`, `cns.skill.registry_validated` each registered explicitly → only these exact strings accepted.

- **G1 (deletion test):** Inline into callers — callers currently rely on the hierarchical rule to accept ad-hoc sub-namespaces. If we delete the hierarchical rule, callers must register every sub-namespace — complexity reappears (but this is the *intended* complexity: making creep visible). Delete the change — creep continues silently. → **PASS G1** (behavior lost = silent creep; complexity reappears = unregistered sub-namespaces fail validation, which is the point).
- **G2 (interface count):** Adds ~16 entries to a `const` array. No new types, no new functions. The `is_canonical` function *simplifies* (removes the recursive branch). → **PASS G2.**
- **G3 (abstraction trace):** The hierarchical rule was a pass-through (it accepted anything under a prefix). The exact-match rule encodes genuine behavior (every sub-namespace is a deliberate registration). → **PASS G3.**

**Gate verdict: PASS.** Constraint force: **Guardrail** (P12 — identity; unregistered spans are anonymous agency).

#### Change 2 — Reorganize by domain/subdomain

Group the ~110 entries into a documented domain/subdomain hierarchy. Proposed structure (illustrative — the full list would be derived from the existing array):

```
cns.acp.*              — Agent Communication Protocol
cns.api.*              — API metering
cns.architecture.seam.* — Seam architecture
cns.authorization      — Authorization
cns.backup.*           — Backup
cns.chat               — Chat
cns.ci.*               — CI / QA
cns.classify.*         — Classification
cns.communication.*    — Communication
cns.condenser          — Condenser
cns.consent            — Consent
cns.consolidation      — Consolidation
cns.contract.*         — Contracts
cns.curation           — Curation
cns.curator.*          — Curator
cns.cybernetics.*      — Cybernetics
cns.deploy.*           — Deploy / Sessions
cns.federation.*       — Federation
cns.gas                — Gas / Energy
cns.goal               — Goal
cns.guard.*            — Guard
cns.heal               — Healing
cns.inference          — Inference
cns.fusion             — Fusion
cns.kata               — Kata
cns.keystore           — Keystore
cns.lora.*             — LoRA training (NEW domain — currently flat)
cns.mcp.media.*       — MCP Media
cns.media              — Media
cns.memory.*           — Memory
cns.multi.*            — Multi-agent
cns.outcome            — Outcome
cns.platform.metric.* — Platform metrics
cns.qa.*               — QA
cns.regulation         — Regulation
cns.runtime.*          — Runtime posture (NEW domain — currently flat)
cns.skill.*            — Skill (NEW subdomain structure — see below)
cns.slo.*              — SLO
cns.sovereignty.*      — Sovereignty
cns.spec               — Spec
cns.storage            — Storage
cns.supply_chain.*     — Supply chain (NEW domain — currently flat)
cns.taxonomy.*         — Attack taxonomy (NEW domain — currently flat)
cns.template           — Template
cns.tool.*             — Tool subsystems
cns.training.provider  — Training providers
cns.userpod.*          — UserPod
cns.semantic.*         — Semantic
cns.variety            — Variety
cns.wallet.*           — Wallet
cns.well.*             — Well
cns.pipeline           — Pipeline (docproc)
```

**Proposed `cns.skill.*` subdomain structure** (the 18 sub-namespaces in use, organized):

```
cns.skill.lifecycle     — skill_activated, skills_loaded, skills_discovered, skill_published
cns.skill.registry     — registry_validated
cns.skill.cascade      — cascade (step execution), compute
cns.skill.convergence  — converged, escalated
cns.skill.budget       — gas_exhausted, gas_alert, rjoule_exhausted, rjoule_alert
cns.skill.frontmatter  — frontmatter_missing (NEW — from R4)
cns.skill.manifest     — manifest_unparseable, manifest_absent (NEW — from R4)
```

This groups the 18 ad-hoc sub-namespaces into 7 documented subdomains. Each subdomain is a cluster of related spans, not a flat list.

- **G1:** Inline into callers — callers currently use flat `cns.skill.cascade` etc. If we delete the subdomain grouping, callers go back to flat — complexity reappears (no way to query "all skill budget spans" without string-matching). Delete the change — flat list continues. → **PASS G1.**
- **G2:** Renames 18 `tracing::target` strings. Adds 0 new types. The `CANONICAL_NAMESPACES` array is reorganized (not grown). → **PASS G2.**
- **G3:** Genuine behavior (subdomain grouping enables variety queries by subdomain). Not a pass-through. → **PASS G3.**

**Gate verdict: PASS.** Constraint force: **Guideline** (deep-module organization).

**Note:** This is a rename across ~18 call sites in `executor.rs` + `skill_impl.rs`. It will break any consumer that string-matches `cns.skill.cascade` (e.g., the `check-cns-canonical.sh` script, which mirrors `is_canonical`). Both must be updated together.

---

## Directive 3 — Proceed as Recommended (R3 + R6 coordination)

The parent audit's open question #3 was: "R6 changes `SkillRegistryIndex::register_skill` from `()` to `Result` — coordinate with R3 in one PR, or sequence?"

**Decision: sequence, not one PR.** R3 (typed `SkillFinding` + `ManifestResolveError`) is a *return-type* change on `resolve_manifest` (a free function). R6 (pool `.expect` → typed error) is a *trait signature* change on `SkillRegistryIndex` / `BundleRegistryIndex`. They touch different surfaces. Sequencing:

1. **R3 first** (Phase B, tasks B4) — introduces `ManifestResolveError`, does not touch the registry traits.
2. **R6 second** (Phase C, task C2) — changes the registry traits to return `Result`. By this point, R3's `ManifestResolveError` exists and can be reused if the trait errors need a finding type.

This avoids a single large PR that changes both a free function and two traits simultaneously.

---

## Directive 4 — Nika Verb Crates Follow-Up Audit

I read all four Nika verb crates in full (`nika-verb-infer`, `nika-verb-exec`, `nika-verb-invoke`, `nika-verb-agent`). Below: the structures that rhyme with hKask's `flowdef` + `knowact`/`wordact`/`renderact` template composition, with falsifiability verdicts.

### 4.1 Nika's 4-verb model vs hKask's 3-template-type model

| Nika verb | What it does | hKask template type | Rhyme |
|---|---|---|---|
| `infer` | One-shot LLM call, optional schema, structured-output retry | `KnowAct` (cognition template) + `WordAct` (prompt template) | `infer` = the LLM call that a `KnowAct` step wraps; `WordAct` = the prompt shaping |
| `exec` | Shell command, argv vs shell form, capture modes | (no direct hKask equivalent — hKask uses MCP tools for shell) | weak rhyme — hKask delegates shell to MCP `hkask-mcp-filesystem` |
| `invoke` | Tool/MCP call, closed namespace (`nika:`/`mcp:`) | `execute` action in `BundleManifestStep` (calls MCP tools) | strong rhyme — both are "call a tool by name with args" |
| `agent` | Multi-turn ReAct loop, tool whitelist, BM25 routing, stall guard | `FlowDef` (the cascade loop in `ManifestExecutor`) | strong rhyme — both are "loop over steps until convergence" |

### 4.2 Key structural lessons (with falsifiability verdicts)

#### L6 — Injected effects (Nika) vs owned effects (hKask)

**Nika:** Every verb takes its effect as an injected `Arc<T>` where `T` is a kernel trait (`ProviderInferDyn`, `ShellRunDyn`, `ToolExecuteDyn`). The verb crate has *no Cargo dep* on the effect implementation. Tests inject mocks; production injects real implementations.

**hKask:** `ManifestExecutor` (in `hkask-templates/src/executor.rs`) holds `Arc<dyn InferencePort>` and `Arc<dyn ToolPort>` — *already injected*. But `SkillLoader` (in `skill_loader.rs`) calls `fs::read_to_string` directly — *not injected*.

**Verdict:** This is the same lesson as L2 (injected reader) from the parent audit. Nika's verb crates are *uniformly* pure-with-injected-effects; hKask is *partially* injected (executor yes, loader no). **SURVIVE** (already in parent audit as R2).

#### L7 — `#[non_exhaustive]` on every public struct

**Nika:** Every public input/output struct (`InferInput`, `InferOutput`, `ExecInput`, `ExecOutput`, `InvokeInput`, `InvokeOutput`, `AgentInput`, `AgentOutput`) is `#[non_exhaustive]` with a `new()` constructor. This allows adding fields in future versions without breaking downstream constructors.

**hKask:** `Skill` (in `hkask-ports/src/registry.rs` L99) is *not* `#[non_exhaustive]`. `BundleManifest`, `BundleManifestStep`, `BundleSkill` — also not `#[non_exhaustive]` (verified by reading `bundle/manifest.rs`).

**Falsifiability:**
- **Popper:** Testable — grep for `#[non_exhaustive]` on public structs in `hkask-templates/src/bundle/` and `hkask-ports/src/registry.rs`.
- **Chamberlin:** Alternative: keep exhaustive structs and require semver-major bumps on field additions. Counterfactual: if hKask does NOT adopt `#[non_exhaustive]`, adding a field to `Skill` breaks every constructor call site.
- **Pearl:** If hKask does NOT adopt — every field addition is a breaking change. Nika adds fields freely (e.g., `InferInput::timeout` was added post-hoc per the doc comment).
- **Platt:** Discriminating test: count downstream constructors of `Skill::new(...)` — if >1, `#[non_exhaustive]` + builder is cheaper than semver-major bumps.

**Verdict:** **SURVIVE.** Constraint force: **Guideline** (API evolution). Recommendation: add `#[non_exhaustive]` to `Skill`, `BundleManifest`, `BundleManifestStep`, `BundleSkill` + ensure each has a `new()` constructor.

#### L8 — Typed verb errors with `nika_code()` + `spec_code()` + `is_transient()`

**Nika:** Each verb has its own `VerbXxxError` enum (`VerbInferError`, `VerbExecError`, `VerbInvokeError`, `VerbAgentError`) with:
- `nika_code()` — the engine numeric code (e.g., `NIKA-451`)
- `spec_code()` — the user-facing spec code (may be the tool's own code, e.g., `NIKA-BUILTIN-FETCH-001`)
- `is_transient()` — whether the error is retryable
- `SpendOnFailure` — the billed round-trips ride the error

**hKask:** `TemplateError` (in `ports.rs`) has 10 variants but no `code()`, no `is_transient()`, no spend tracking. `ManifestLoadError` has 2 variants, same gaps.

**Falsifiability:**
- **Popper:** Testable — grep for `fn nika_code` / `fn spec_code` / `fn is_transient` in `hkask-templates/src/ports.rs`.
- **Chamberlin:** Alternative: keep `TemplateError` as-is, add a `code()` method later. Counterfactual: without `code()`, the finding-code vocabulary (R3 open question) cannot be machine-readable.
- **Pearl:** If hKask does NOT adopt — errors are strings; `kask explain`-style fix pointers impossible; the finding-code vocabulary (R3) has no carrier.
- **Platt:** Discriminating test: does any consumer need to branch on error *class* (not just error *message*)? The `error_handling` block in manifests (`on_gas_exceeded: abort`, `on_timeout: retry`) implies yes — but it branches on *condition*, not on *error variant*.

**Verdict:** **SURVIVE.** Constraint force: **Guardrail** (enables R3's finding-code vocabulary). Recommendation: add `code()`, `is_transient()` to `TemplateError` and `ManifestLoadError`. This composes with R1 (typed errors) and R3 (findings).

#### L9 — `CANCEL SAFETY` documented on every async verb

**Nika:** Every verb's `run()` doc has a `CANCEL SAFETY:` paragraph stating what happens if the future is dropped mid-call. Example (from `infer`): "cancel-safe at the provider transport (kernel contract) — no state is mutated; dropping the future mid-call abandons the request."

**hKask:** `ManifestExecutor::execute_manifest` (in `executor.rs`) has no `CANCEL SAFETY` doc. The function holds `taint_labels: Mutex` (L194, L1301) — if the future is dropped mid-call, the mutex is released (poison-free), but the cascade state (gas used, iteration count) is in local variables and lost.

**Falsifiability:**
- **Popper:** Testable — grep for `CANCEL SAFETY` in `hkask-templates/src/executor.rs`.
- **Chamberlin:** Alternative: rely on Rust's general async-cancel-safety rules. Counterfactual: without explicit docs, a caller cannot know whether dropping `execute_manifest` mid-cascade leaves the registry in a consistent state.
- **Pearl:** If hKask does NOT adopt — cancel-safety is implicit and per-caller. Nika's explicit docs make it a contract.
- **Platt:** Discriminating test: does any caller drop `execute_manifest` mid-cascade? The REPL turn loop might (user interrupts). If yes, the contract matters.

**Verdict:** **SURVIVE** (weak). Constraint force: **Guideline** (documentation discipline). Recommendation: add `CANCEL SAFETY:` paragraphs to `execute_manifest` and any other async public function in the skill system.

#### L10 — The `agent` verb's stall guard + BM25 routing

**Nika:** The `agent` verb has a `Guard` (stall detection over action+observation signatures, with a bounded Reflexlexion nudge before `NIKA-467`) and a `ToolRouter` (BM25 active discovery over the whitelisted tool universe, MCP-Zero-style).

**hKask:** `ManifestExecutor` has convergence checking (`manifest.convergence.threshold`, `max_iterations`) but no stall detection (it doesn't detect "the cascade is repeating the same step") and no tool routing (it executes `step.mcp` directly).

**Falsifiability:**
- **Popper:** Testable — grep for `stall` / `signature` / `BM25` / `router` in `hkask-templates/src/executor.rs`.
- **Chamberlin:** Alternative: keep the current convergence-only loop. Counterfactual: a FlowDef skill that loops on the same step (e.g., a broken `loop_target`) will run until `max_iterations` with no early stop.
- **Pearl:** If hKask does NOT adopt — broken loops burn gas until `max_iterations`. Nika's stall guard catches them early.
- **Platt:** Discriminating test: write a FlowDef that loops on step 1 forever with `max_iterations: 100` — does hKask detect the stall? (Inferred: no — it runs 100 iterations.)

**Verdict:** **SURVIVE** (medium). Constraint force: **Hypothesis** (cybernetic variety — Ashby's Law: the loop needs a model of its own behavior to detect when it's stuck). Recommendation: add a stall guard to `ManifestExecutor` that hashes `(step.ordinal, step.action, step.template_ref)` and detects repetition within a window. This is a *new feature*, not a refactor — it should go through the essentialist 3-gate separately.

### 4.3 Summary of follow-up findings

| Lesson | Source | Verdict | Constraint force | Action |
|---|---|---|---|---|
| L6 | Injected effects | SURVIVE (already R2) | Guardrail | R2 in parent plan |
| L7 | `#[non_exhaustive]` on public structs | SURVIVE | Guideline | New task D1 |
| L8 | Typed verb errors with `code()`/`is_transient()` | SURVIVE | Guardrail | New task D2 (composes with R1, R3) |
| L9 | `CANCEL SAFETY` docs | SURVIVE (weak) | Guideline | New task D3 |
| L10 | Stall guard + BM25 routing | SURVIVE (medium) | Hypothesis | New task D4 (new feature — separate gate) |

---

## Updated Task Plan (incorporating directives 1-4)

The parent plan (Phases A/B/C, tasks A1-C3) stands. Add:

### Phase D — Nika Verb Follow-Up (from directive 4)

---

#### D1 — Add `#[non_exhaustive]` to skill-system public structs

- **slice_id:** D1
- **Title:** Add non_exhaustive to Skill/BundleManifest/BundleManifestStep/BundleSkill
- **AC:**
  1. `Skill`, `BundleManifest`, `BundleManifestStep`, `BundleSkill` are `#[non_exhaustive]`.
  2. Each has a `new()` constructor (or existing constructors are preserved).
  3. `cargo test --workspace` passes (no downstream constructor breaks — all callers are in-workspace).
- **Verification:** `cargo test --workspace && cargo clippy --workspace -- -D warnings`
- **Dependencies:** none (independent of A-C)
- **Files:** `crates/hkask-ports/src/registry.rs`, `crates/hkask-templates/src/bundle/manifest.rs`
- **Scope:** S

---

#### D2 — Add `code()` and `is_transient()` to `TemplateError` and `ManifestLoadError`

- **slice_id:** D2
- **Title:** Add code() and is_transient() to skill-system error enums
- **AC:**
  1. `TemplateError` has `pub fn code(&self) -> &'static str` returning a stable code (e.g., `"HKASK-SKILL-001"` for `SkillLoad`, `"HKASK-SKILL-002"` for `Frontmatter`).
  2. `TemplateError` has `pub fn is_transient(&self) -> bool` (true for `Database`, `Inference`; false for `NotFound`, `Validation`, `PathTraversal`).
  3. `ManifestLoadError` has the same two methods.
  4. `cargo test --workspace` passes.
- **Verification:** `cargo test --workspace`
- **Dependencies:** A1 (the new variants must exist first)
- **Files:** `crates/hkask-templates/src/ports.rs`, `crates/hkask-templates/src/manifest_loader.rs`
- **Scope:** S

---

#### D3 — Add `CANCEL SAFETY` docs to `execute_manifest` and async skill-system functions

- **slice_id:** D3
- **Title:** Document cancel-safety on async skill-system public functions
- **AC:**
  1. `ManifestExecutor::execute_manifest` has a `CANCEL SAFETY:` paragraph in its rustdoc.
  2. Any other `async pub fn` in `hkask-templates/src/executor.rs` has the same.
  3. `cargo doc -p hkask-templates` passes with no broken intra-doc links.
- **Verification:** `cargo doc -p hkask-templates`
- **Dependencies:** none
- **Files:** `crates/hkask-templates/src/executor.rs`
- **Scope:** XS

---

#### D4 — Add stall guard to `ManifestExecutor` (new feature — separate essentialist gate)

- **slice_id:** D4
- **Title:** Stall detection in ManifestExecutor cascade loop
- **AC:**
  1. `execute_manifest` detects when the same `(step.ordinal, step.action, step.template_ref)` tuple repeats within a window of N iterations.
  2. On stall detection, emits `cns.skill.stall_detected` and either escalates or nudges (configurable).
  3. New test: a FlowDef that loops on step 1 forever with `max_iterations: 100` is caught within ≤5 iterations.
- **Verification:** `cargo test -p hkask-templates` (new test for stall detection)
- **Dependencies:** B4 (the cascade loop must be typed-error-aware before adding stall logic)
- **Files:** `crates/hkask-templates/src/executor.rs`
- **Scope:** M
- **Note:** This is a *new feature*, not a refactor. It must pass its own essentialist 3-gate before implementation. The gate is not run here — flag for separate review.

---

### Phase E — Lexicon Rationalization (from directive 1)

This phase depends on your answer to the open question in §1.3 (closed vocabulary vs open tag list). Tasks below assume **Option B** (closed vocabulary). If you choose Option A (delete) or Option C (replace with typed enums), this phase changes.

---

#### E1 — Audit existing `lexicon_terms` values across all manifests

- **slice_id:** E1
- **Title:** Enumerate all lexicon_terms values in registry/manifests and registry/templates
- **AC:**
  1. A report listing every distinct `lexicon_terms` value across `registry/manifests/*.yaml` and `registry/templates/*/manifest.yaml`.
  2. The report notes which values are already covered by `TemplateType` / `functional_role` / `category`.
  3. The report is written to `tasks/lexicon-audit.md`.
- **Verification:** manual review
- **Dependencies:** none (read-only)
- **Files:** `tasks/lexicon-audit.md` (new file)
- **Scope:** S

---

#### E2 — Promote the lexicon to a `LexiconTerm` enum (if E1 shows a small closed set)

- **slice_id:** E2
- **Title:** Add LexiconTerm enum and validate lexicon_terms at parse time
- **AC:**
  1. `LexiconTerm` enum exists in `hkask-types` with one variant per distinct value from E1.
  2. `RegistryEntry.lexicon_terms` and `BundleSkill.lexicon_terms` are `Vec<LexiconTerm>` (not `Vec<String>`).
  3. Unknown lexicon values produce a parse error (not silent drop).
  4. `cargo test --workspace` passes.
- **Verification:** `cargo test --workspace`
- **Dependencies:** E1
- **Files:** `crates/hkask-types/src/lib.rs` (new `LexiconTerm` type), `crates/hkask-ports/src/registry.rs`, `crates/hkask-templates/src/bundle/manifest.rs`, `crates/hkask-services-skill/src/audit.rs`
- **Scope:** M
- **Note:** This is a *schema change* — it will break any manifest with a `lexicon_terms` value not in the enum. Gate on E1's report.

---

### Phase F — CNS Namespace Reorganization (from directive 2)

---

#### F1 — Register all `cns.skill.*` sub-namespaces explicitly in `CANONICAL_NAMESPACES`

- **slice_id:** F1
- **Title:** Register cns.skill subdomains explicitly
- **AC:**
  1. `CANONICAL_NAMESPACES` includes explicit entries for every `cns.skill.*` sub-namespace in use (the 18 from §2.2, organized into the 7 subdomains from §2.3).
  2. `is_canonical` is changed from hierarchical-prefix to exact-match (with a deprecation period if needed).
  3. `scripts/check-cns-canonical.sh` is updated to mirror the new rule.
  4. `cargo test -p hkask-types` passes (the proptest `all_canonical_namespaces_parse` still passes).
- **Verification:** `cargo test -p hkask-types && scripts/check-cns-canonical.sh`
- **Dependencies:** none
- **Files:** `crates/hkask-types/src/event.rs`, `scripts/check-cns-canonical.sh`
- **Scope:** M

---

#### F2 — Rename `cns.skill.*` tracing targets to the subdomain structure

- **slice_id:** F2
- **Title:** Rename cns.skill tracing targets to subdomains
- **AC:**
  1. All `tracing::target: "cns.skill.cascade"` etc. in `executor.rs` and `skill_impl.rs` are renamed to the subdomain structure (e.g., `cns.skill.cascade` stays, `cns.skill.converged` → `cns.skill.convergence.converged`, `cns.skill.gas_exhausted` → `cns.skill.budget.gas_exhausted`).
  2. `cargo test --workspace` passes.
  3. `grep -r 'cns\.skill\.' crates/ | grep -v 'cns\.skill\.\(lifecycle\|registry\|cascade\|convergence\|budget\|frontmatter\|manifest\)'` returns zero matches (all targets are in a subdomain).
- **Verification:** `cargo test --workspace && grep -r 'cns\.skill\.' crates/ | grep -v 'cns\.skill\.\(lifecycle\|registry\|cascade\|convergence\|budget\|frontmatter\|manifest\)'`
- **Dependencies:** F1 (the new subdomains must be registered first)
- **Files:** `crates/hkask-templates/src/executor.rs`, `crates/hkask-services-skill/src/skill_impl.rs`
- **Scope:** M

---

#### F3 — Add CNS namespace creep defense: CI gate for unregistered sub-namespaces

- **slice_id:** F3
- **Title:** CI gate rejecting unregistered cns.* sub-namespaces
- **AC:**
  1. A script `scripts/check-cns-creep.sh` scans all `tracing::target: "cns.*"` strings in `crates/` and verifies each is in `CANONICAL_NAMESPACES` (exact match, not hierarchical).
  2. The script is wired into `.github/workflows/ci.yml`.
  3. The script passes on the current codebase (after F1+F2).
- **Verification:** `scripts/check-cns-creep.sh && .github/workflows/ci.yml` (dry run)
- **Dependencies:** F1, F2
- **Files:** `scripts/check-cns-creep.sh` (new), `.github/workflows/ci.yml`
- **Scope:** S

---

### Updated Phase Summary

| Phase | Tasks | Scope | Checkpoint |
|---|---|---|---|
| A — Foundation (typed errors + finding type) | A1, A2, A3 | XS, S, S | After A: `cargo test --workspace` |
| B — Core (purity, single-voice, no silent defaults) | B1, B2, B3, B4 | M, S, S, M | After B: `cargo test --workspace` + `check-string-errors.sh` |
| C — Polish (encapsulation, panic-safety, schema strictness) | C1, C2, C3 | M, M, S | After C: `cargo test --workspace` + `clippy -- -D warnings` |
| D — Nika Verb Follow-Up | D1, D2, D3, D4 | S, S, XS, M | After D: `cargo doc` + `cargo test` |
| E — Lexicon Rationalization | E1, E2 | S, M | After E: `cargo test --workspace` |
| F — CNS Namespace Reorganization | F1, F2, F3 | M, M, S | After F: `check-cns-canonical.sh` + `check-cns-creep.sh` |

**Total: 20 tasks** (13 from parent + 7 new). Phases A-C are the parent plan; D-F are new.

**Dependency graph (simplified):**
- A1 → A2, A3, D2
- A2 → B1, B2, B3
- A3 → B4
- B1 → B2, B3
- B4 → D4
- C1, C3, D1, D3, E1, F1 — independent
- C2 — coordinate with B4 (per directive 3)
- E1 → E2
- F1 → F2 → F3

---

## Open Questions for the Operator

1. **Lexicon (directive 1):** Is the lexicon meant to be (a) a closed vocabulary of domain verbs, or (b) an open tag list for search? §1.3 recommends Option B (closed vocabulary) if (a), Option A (delete + full-text search) if (b). E1 will produce the data to decide.
2. **CNS exact-match (directive 2, F1):** Changing `is_canonical` from hierarchical to exact-match is a *breaking* change for any consumer that relies on ad-hoc sub-namespaces. Do you want a deprecation period (warn-on-unregistered for one release, then enforce), or a hard cutover?
3. **Stall guard (directive 4, D4):** This is a new feature, not a refactor. Do you want it gated separately (its own essentialist 3-gate + design review), or folded into the existing plan?
4. **`#[non_exhaustive]` (D1):** This is technically a breaking change for any *external* consumer that constructs `Skill { ... }` with struct literal syntax. Are there external consumers, or is `Skill` only constructed in-workspace? (Inferred: in-workspace only, since `hkask-ports` is an internal crate — but verify.)

---

**End of follow-up audit.**
