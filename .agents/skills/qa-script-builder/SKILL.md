---
name: qa-script-builder
visibility: public
description: "Design and generate autonomous QA pipeline manifests for the hKask QA system. Walks through a structured persona→discover→design→generate→validate pipeline: generates diverse testing scenarios from persona + goal, maps testing intent to the QA system's capabilities (fuzz, classify, branch, loop, auto-repair), designs the branching state machine, and produces a valid YAML manifest for `kask qa run --script`. Use when the user says 'build a QA script', 'create a QA pipeline', 'design a fuzz workflow', or 'generate a QA manifest'."
---

# QA Script Builder

You are a QA pipeline designer. Your job is to translate a user's quality-assurance intent into a **QA script manifest** — a YAML file that the hKask QA system's `QaScriptRunner` can execute autonomously via `kask qa run --script`.

## What You Build

A QA script manifest is a state machine expressed in YAML. Steps can:
- Run shell commands and branch on exit codes
- Send passages to LLM classifiers and branch on confidence levels
- Loop with retry/backoff until a condition is met or max iterations exhausted

The manifest controls: gas budgets, CNS observability spans, error handling, and audit trails.

## Registry Templates

This skill's runtime templates live in `registry/templates/qa-script-builder/`:

| Template | Type | Purpose |
|----------|------|---------|
| `qa-persona.j2` | KnowAct | Phase 0: Generate diverse QA testing scenarios from persona + goal using Falstaffian perspective rotation and grill-me adversarial probing |
| `qa-discover.j2` | KnowAct | Phase 1: Discover the test surface — crate, failure modes, existing fuzz coverage, what needs testing (accepts persona scenario or raw intent) |
| `qa-design.j2` | KnowAct | Phase 2: Design the branching state machine from testing intent to step topology |
| `qa-generate.j2` | KnowAct | Phase 3: Generate the complete YAML manifest from the designed topology |
| `qa-validate.j2` | KnowAct | Phase 4: Validate the manifest against schema, check branch referential integrity, ensure classifier configs exist |

The SKILL.md (this file) teaches the Zed coding agent the QA script design methodology. The .j2 templates are executable process steps the hKask runtime invokes during `kask chat` sessions.

## The QA System You're Targeting

Before designing any script, you must understand the capabilities of the runner.

### Step Actions

| Action | What It Does | Key Fields |
|--------|-------------|------------|
| `run_command` | Executes `sh -c "<command>"`, branches on exit code 0 (success) vs non-zero (failure) | `command`, `branching` |
| `classify` | Sends a passage to an LLM classifier, branches on confidence level | `classifier` (name of classifier config), `branching` |
| `loop` | Repeats a command or classify action until a branch condition matches or `max_iterations` hit | `max_iterations`, `iteration_delay_secs`, `command` or `classifier` |

### Branch Conditions

| Condition | Triggered When | Applicable Actions |
|-----------|---------------|-------------------|
| `success` | Shell command exit code = 0 | `run_command`, `loop` |
| `failure` | Shell command exit code ≠ 0 | `run_command`, `loop` |
| `high_confidence` | Classifier returned confidence ≥ 0.95 | `classify` |
| `medium_confidence` | Classifier returned confidence 0.70–0.949 | `classify` |
| `low_confidence` | Classifier returned confidence > 0 and < 0.70 | `classify` |
| `flake` | Classifier returned `is_flake: true` | `classify` |
| `unparseable` | Classifier returned non-JSON or confidence = 0 | `classify` |
| `loop_exhausted` | Loop reached max_iterations without matching any branch | `loop` |

If no branch condition matches, the runner uses `default_next`. If neither is set, it advances linearly.

### Gas Budget

Every script can declare a `gas` section that tracks estimated token costs:
- `cap`: maximum gas units (default: 15000)
- `cost_per_token`: per-token cost for estimation (default: 0.25)
- `alert_threshold`: fraction of cap that triggers a warning (default: 0.7)
- `hard_limit`: if true, script aborts when cap exceeded (default: true)

When the runner hits the gas cap, it errors with `GasExceeded` unless `hard_limit: false`.

### Algedonic Signalling (CNS Integration)

QA scripts don't just observe — they **signal**. Every classify step and terminal step can raise a direct algedonic alert that flows through the CNS Cybernetics Loop into the Curation Loop's inbox, where the Curator reviews it with the human operator. This is per-failure, per-classification, immediate — not aggregated variety deficit.

**The signal path:**
```
QA classify step → RuntimeAlert → alerts_tx channel → CurationInput::Alert
    → Curation Loop inbox → CuratorAgent → human review → (training_curate_feedback)
```

**Alert configuration per step:**

| Field | Purpose | Default |
|-------|---------|--------|
| `escalate_on` | Which classification outcomes trigger alerts | `[]` (no alerts) |
| `severity_map` | Outcome → severity: `critical`, `warning`, `info` | `{}` (all info) |
| `domain` | Alert domain for correlation (e.g., `qa.hkask-types.fuzz`) | `qa.<script-id>` |
| `threshold` | Escalate after this many failures in the domain | 3 |
| `cooldown_secs` | Don't re-escalate within this window | 300 |

**Global alert section in manifest:**

```yaml
alert:
  enabled: true                     # Master switch
  default_domain: qa.<script-id>    # Fallback domain
  default_threshold: 5              # Escalate after N failures
  default_cooldown_secs: 600        # Suppress repeat alerts
  escalate_to_curator: true         # Route alerts to Curation Loop
```

**Critical:** Alerts are active escalation, not passive logging. A `cns_span` string on a step is a tracing target (goes to logs). An `alert` config on a step raises a `RuntimeAlert` that reaches the Curator. They are orthogonal:
- `cns_span` → tracing/logging (always works)
- `alert` → algedonic escalation (requires wired alerts_tx channel)

### Existing Classifier Configs

| Classifier | Purpose | Model |
|-----------|---------|-------|
| `qa-triage` | Diagnose Rust test failures (confidence, root_cause, proposed_fix, is_flake) | Gemma 4 26B A4B |
| `qa-feedback` | Suggest fuzz targets from surviving mutants | Gemma 4 26B A4B |

Both live at `registry/classify/<name>.yaml` and require `DEEPINFRA_API_KEY`.

### Runner Capabilities & Limitations

**Capabilities:**
- Execute shell commands and branch on exit codes
- Classify passages via LLM and branch on confidence levels
- Loop with retry/backoff
- Raise algedonic alerts on classification outcomes (direct to Curator)
- Emit CNS tracing spans for observability
- Track and enforce gas budgets

**Limitations:**
- Cannot modify files (no file I/O beyond shell commands you write)
- Cannot call `kask` subcommands (runs raw shell)
- Cannot pass data between steps (no variables, no state sharing)
- Cannot conditionally skip steps (branching only, no `if` expressions)
- Cannot spawn sub-scripts (no nested manifest execution)
- Cannot run classifiers without an API key (`DEEPINFRA_API_KEY`)

## The 5-Phase Pipeline

### Phase 0: Persona (Scenario Generation)

Generate diverse testing scenarios from a user persona and goal. This phase produces a `scenario_set` — a family of testing intents that stress-test different MCP servers and central services from different angles.

**Input:** A persona description ("You are an SRE responsible for MCP server reliability") and a goal ("I want to monitor flake rates across all services").

**Process:** Apply Falstaffian perspective rotation to generate diversity:
1. **Obvious path** — the most direct interpretation of the goal
2. **Shadow path** — the failure mode the persona fears but isn't saying
3. **Adjacent path** — a neighboring concern one hop away
4. **Inversion** — the complementary scenario (verify non-failures, not detect failures)
5. **Wildcard** (optional) — a capability the persona is overlooking

Then apply grill-me adversarial probing to each scenario: "What assumption might not hold?" "What's the smallest input that could break this?" Output hardened scenarios with `grill_hardened: true`.

**Output:** A `scenario_set` array of 3–5 scenarios, each with:
- `user_intent` — natural-language testing intent (feeds into Phase 1)
- `testing_angle` — unique angle (fuzz, contract, flake, alert, CNS, gas, loop, repair)
- `failure_mode` — what they're testing against
- `suggested_tool` — which tool produces failures
- `alert_posture` — escalation severity mapping
- `gas_environment` — CI-tight or local-generous
- `stress_target` — which MCP server or central service this exercises

**When to skip Phase 0:** When the user already has a specific testing intent and doesn't need scenario diversity.

### Phase 1: Discover

Map the user's testing intent (or a persona-generated scenario) to the QA system's capabilities. Ask:

1. **What are we testing?** Crate? Function? Behavior? Invariant?
2. **What failure modes matter?** Panics? Assertions? Timeouts? Flakes? Logic errors?
3. **What tool produces the failures?** `cargo bolero test`? `cargo test`? `cargo mutants`? Custom script?
4. **What should happen on failure?** Auto-repair high-confidence? Escalate medium? Retry flakes? Log and move on?
5. **What severity triggers escalation?** High-confidence failures → critical alert? Medium → warning? Flakes after retry exhaustion → critical? Per-outcome severity mapping.
6. **What's the gas budget?** Is this CI (tight budget) or local (generous)?
7. **What alert domain and thresholds?** Namespace for this script's alerts (`qa.<crate>.fuzz`). How many failures before the Curator is alerted? Cooldown between re-escalations?
8. **Are there existing fuzz targets / classifier configs to reuse?**

### Phase 2: Design

Translate discovery into a step topology — a directed graph where nodes are steps and edges are branch conditions.

Key design decisions:
- **Entry point**: Which command produces the test output?
- **Classification**: Do we classify failures? With which classifier?
- **Branching topology**: Where does each confidence level route?
- **Alert topology**: Which outcomes trigger alerts? At what severity? With what domain?
- **Terminal states**: Which steps have no outgoing branches (script ends there)?
- **Loops**: Do we retry flakes? With what delay and max iterations?
- **Error handling**: What happens on gas exceeded? On command timeout?

Draw the topology mentally before generating YAML:
```
Step 1 (fuzz) ──success──> Step 2 (classify)
  │ failure                       │
  └──> Step N (report)      ┌─────┼─────┬──────┐
                            │     │     │      │
                         high   med   low   flake
                            │     │     │      │
                            ▼     ▼     ▼      ▼
```

### Phase 3: Generate

Produce the YAML manifest with all sections: `manifest`, `gas`, `steps`, `error_handling`, `cns`, `audit`.

Every step ordinal must be unique. Every branch target must reference an existing ordinal. Classifier names must match existing configs in `registry/classify/`.

### Phase 4: Validate

Check the generated manifest:

| Check | Severity | Description |
|-------|----------|-------------|
| All branch targets resolve | Error | Every `branching` value must reference a step ordinal that exists |
| Classifier configs exist | Error | Every `classifier` field must reference a known classifier config name |
| Alert escalate_on targets valid outcomes | Error | Every outcome in `escalate_on` must be a valid branch condition for that action type |
| Alert domains are namespaced | Warning | Alert domains should follow `qa.<crate>.<tool>` convention |
| Terminal steps with alerts specify severity | Warning | Terminal steps that escalate should declare severity in `severity_map` |
| No orphan steps | Warning | Every step should be reachable from the entry point |
| Gas budget is sensible | Warning | For CI scripts, cap should reflect actual token estimates |
| No duplicate ordinals | Error | Every step must have a unique ordinal |
| At least one terminal state | Warning | Script must be able to terminate |

## Common Script Patterns

### Pattern A: Fuzz → Classify → Alert → Auto-repair or Escalate

The canonical pattern. Run fuzz tests, classify failures, raise direct algedonic alerts per outcome, auto-repair high-confidence, escalate the rest.

```
Step 1: cargo bolero test
  → success: Step 2
  → failure: Step 5 (infrastructure failure)

Step 2: classify (qa-triage)
  → high_confidence: Step 3 (auto-repair)
  → medium_confidence: Step 4 (escalate)
  → low_confidence: Step 4
  → flake: Step 6 (retry loop)
  alert:
    escalate_on: [high_confidence, medium_confidence]
    severity_map:
      high_confidence: critical
      medium_confidence: warning
      flake: info
    domain: qa.hkask-types.fuzz
    threshold: 3

Step 3: echo "auto-repair triggered"     [terminal]
Step 4: echo "escalate to human"          [terminal]
Step 5: echo "fuzz infrastructure failure" [terminal]
Step 6: loop (retry up to 3 times)
  → success: Step 1 (re-run fuzz)
  → loop_exhausted: Step 4 (escalate)
  alert:
    escalate_on: [loop_exhausted]
    severity_map:
      loop_exhausted: critical
    domain: qa.hkask-types.fuzz
```

### Pattern B: Contract Verification Gate

Run contract tests, classify failures, pass/fail gate.

```
Step 1: cargo test --contract
  → success: Step 3 (pass)
  → failure: Step 2 (classify)

Step 2: classify (qa-triage)
  → high_confidence: Step 4 (auto-fix)
  → medium_confidence: Step 5 (escalate)
  → low_confidence: Step 5

Step 3: echo "all contracts hold"        [terminal - PASS]
Step 4: echo "auto-fix violation"        [terminal - FAIL with fix]
Step 5: echo "human review needed"       [terminal - FAIL ambiguous]
```

### Pattern C: Mutation → Fuzz Suggestion

Run mutation testing, suggest fuzz targets for survivors.

```
Step 1: cargo mutants --timeout 60
  → success: Step 3 (clean)
  → failure: Step 2 (suggest fuzz targets)

Step 2: classify (qa-feedback)
  → [terminal, suggestions printed]

Step 3: echo "no surviving mutants"      [terminal]
```

## Anti-Patterns

| Anti-Pattern | Why It's Wrong | Fix |
|-------------|---------------|-----|
| **Infinite loops** | Loop without `loop_exhausted` branch and no terminal condition | Always add a `loop_exhausted` branch or ensure the loop condition can be met |
| **Unreachable steps** | Steps with ordinals above the max branch target | Reorder or remove unreachable steps |
| **Classifier without API key** | `classify` step without a reachable API key config | Warn user — script will fail at runtime; suggest `run_command` dry-run mode |
| **Under-specified terminals** | Terminal steps that don't tell the user what happened | Every terminal step should print context (what was decided, what to do next) |
| **Gas budget too tight** | Gas cap lower than estimated classification cost | Estimate tokens: ~200 in + ~300 out per classify step, multiply by step count |

## Workflow

1. **User describes testing intent** — "I want to fuzz hkask-types and auto-repair panics"
2. **Phase 1: Discover** — Load the test surface (crate, existing fuzz targets, classifier configs). Name ambiguities.
3. **Phase 2: Design** — Present the branching topology as a mermaid diagram or text graph. Ask: "Does this flow match what you want?"
4. **Phase 3: Generate** — Produce the YAML manifest. Include the `manifest.id`, `manifest.description`, and all steps.
5. **Phase 4: Validate** — Run validation checks, surface warnings, offer to fix issues.
6. **User saves and runs** — `kask qa run --script <path>/<script-name>.yaml`
