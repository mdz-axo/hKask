---
title: "hKask QA System Guide"
audience: [developers, operators]
last_updated: 2026-07-02
version: "0.31.0"
status: "Active"
domain: "Quality Assurance"
mds_categories: [domain, lifecycle, curation]
---

# hKask QA System Guide

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Running QA Scripts](#running-qa-scripts)
4. [Manifest Format](#manifest-format)
   - [Step Actions](#step-actions)
   - [Branching](#branching)
   - [Writing Custom Scripts](#writing-custom-scripts)
5. [Classification Service](#classification-service)
   - [Classifier Configs](#classifier-configs)
   - [Diagnosis Schema](#diagnosis-schema)
6. [CNS Integration](#cns-integration)
7. [Test Harness Components](#test-harness-components)
8. [Recipes](#recipes)

---

## 1. Overview

The hKask QA system runs YAML-defined test manifests through
`hkask_test_harness::qa_script::run_script()`. Each manifest executes
`cargo test` commands, optionally classifies failures via Gemma 4 26B,
and routes to terminal states based on branch conditions.

| Command | What It Does |
|---------|-------------|
| `kask qa list` | List all available QA manifests |
| `kask qa run --script <path>` | Execute a QA manifest |
| `cargo test -p hkask-test-harness -- qa_script` | Run all QA tests |

### Principle Grounding

| Principle | How the QA system satisfies it |
|-----------|-------------------------------|
| **P4** Clear Boundaries | Gas budgets cap LLM costs; `classifier_unavailable` graceful degradation |
| **P5** Essentialism | One module (`qa_script.rs`), one public function (`run_script`) |
| **P8** Semantic Grounding | CNS spans emitted on start/complete/error; classify results carry confidence |
| **P9** Homeostatic Self-Regulation | Branching adapts to test outcomes; gas enforcement prevents runaway |

---

## 2. Architecture

```
┌──────────────────────────────────────────────────────────────────────────┐
│  CLI Surface (hkask-cli)                                                  │
│                                                                           │
│  kask qa run --script <manifest>                                          │
│  kask qa list                                                             │
│       │                                                                    │
│       ▼                                                                    │
│  ┌──────────────────────────────────────────────────────────────┐        │
│  │  commands/qa.rs — thin dispatch to hkask-test-harness        │        │
│  └─────────────────────────┬────────────────────────────────────┘        │
│                            │                                              │
│  ┌─────────────────────────┴──────────────────────────────────┐         │
│  │  hkask-test-harness/src/qa_script.rs                        │         │
│  │  - run_script() → parse YAML, validate, execute steps       │         │
│  │  - run_command steps → shell out (5-min timeout)             │         │
│  │  - classify steps → Gemma 4 26B via DeepInfra API            │         │
│  │  - loop steps → retry with max_iterations guard              │         │
│  │  - mcp_tool steps → stub (routes to failure)                 │         │
│  │  - gas enforcement → hard_limit on gas.cap                   │         │
│  │  - CNS spans → cns.qa.{repair_attempted,verified,exhausted}  │         │
│  └────────────────────────────────────────────────────────────┘         │
└──────────────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
YAML Manifest ──> load_manifest() + validate()
                      │
                      ▼
              run_script() — walk steps by ordinal
                      │
         ┌────────────┼────────────┐
         ▼            ▼            ▼
    run_command   classify      loop
         │            │            │
         │    ┌───────┴───────┐    │
         │    ▼               ▼    │
         │  high_conf     medium   │
         │    │               │    │
         ▼    ▼               ▼    ▼
      branch to specified ordinal (or advance linearly)
                      │
                      ▼
              QaScriptReport
```

---

## 3. Running QA Scripts

QA manifests are executed via `hkask_test_harness::qa_script::run_script()` —
a Rust library function that parses YAML manifests and executes steps with
branching, classifier triage, loop retry, and gas enforcement.

### 3.1 Via cargo test (works today)

```bash
# Run all QA integration tests
cargo test -p hkask-test-harness -- qa_script

# Run a specific manifest end-to-end
cargo test -p hkask-test-harness -- qa_script::tests::run_comm_integration_gate
```

### 3.2 Planned CLI (not yet built)

The `kask qa run --script <path>` CLI subcommand is planned but not yet
implemented. Once built, usage will be:

```bash
# (planned, not yet available)
kask qa run --script registry/manifests/qa-comm-integration-gate.yaml
```

### 3.3 Fuzz Triage (planned)

`kask qa triage` and `kask qa suggest-fuzz` are specified but not yet
built. These will parse bolero/cargo-mutants output and classify failures
via the LLM classifier.

### 3.4 Executable manifests today

Four manifests are executable via `cargo test` without any API keys:

| Manifest | Tests | Status |
|----------|-------|--------|
| `qa-comm-integration-gate` | 5 | ✅ Executable |
| `qa-condenser-health-check` | 11 | ✅ Executable |
| `qa-keystore-security-gate` | 16 | ✅ Executable |
| `qa-memory-privacy-boundary` | 6 | ✅ Executable |

Classify steps gracefully degrade through `classifier_unavailable` when
`DI_API_KEY` is not set, routing to the WARN terminal step.

---

## 4. Manifest Format

A QA script manifest is a YAML file following the `QaScriptManifest` schema. Here's the full structure:

```yaml
manifest:           # Required: metadata
  id: my-script     #   Unique identifier (slug)
  name: My Script   #   Human-readable name
  description: >    #   What the script does
    Description.
  editor: system    #   Author (optional)
  visibility: Public #  Public or Private (optional)

gas:                # Optional: cost controls
  cap: 50000        #   Max gas units (default: 15000)
  alert_threshold: 0.8  # Alert at this fraction (default: 0.7)
  hard_limit: true  #   Abort when exceeded (default: true)

inputs:             # Optional: declared inputs
  - name: workspace_root
    required: false
    description: "Path to workspace"

steps:              # Required: ordered steps
  - ordinal: 1
    action: classify
    classifier: qa-triage
    description: "What to classify"
    branching:
      high_confidence: 3
      medium_confidence: 4
      low_confidence: 5
    default_next: 2
    cns_span: cns.qa.my_script.classify_step

  - ordinal: 2
    action: run_command
    command: "cargo test 2>&1"
    description: "Run tests"
    branching:
      success: 4
      failure: 3

cns:                # Optional: CNS observability
  emit_spans: true
  span_namespace: cns.qa.my_script

audit:              # Optional: audit trail
  enabled: true
  log_level: info
```

### 4.1 Step Actions

| Action | Description | Required Fields |
|--------|-------------|----------------|
| `classify` | Send passage to LLM classifier, branch on confidence | `classifier`, `description` |
| `run_command` | Execute shell command, branch on exit code | `command` (defaults to `true`) |
| `loop` | Repeat a command or classify until a branch condition matches or max iterations | `max_iterations` (default: 5), `command` or `classifier` |
| *(any other)* | Treated as passthrough — always "success" | — |

### 4.2 Branching

Each step can define a `branching` map that routes execution to a target ordinal:

| Outcome | Triggered When |
|---------|---------------|
| `high_confidence` | Classify returned confidence ≥ 0.95 |
| `medium_confidence` | Classify returned confidence 0.70–0.95 |
| `low_confidence` | Classify returned confidence < 0.70 but > 0 |
| `flake` | Classify returned `is_flake: true` |
| `unparseable` | Classify returned non-JSON or confidence = 0 |
| `success` | Shell command exited with code 0 |
| `failure` | Shell command exited with non-zero code |
| `loop_exhausted` | Loop reached `max_iterations` without matching a branch |

If no branch condition matches, `default_next` is used. If neither is set, the
script advances to the next ordinal linearly.

### 4.3 Writing Custom Scripts

**Pattern: Fuzz → Classify → Auto-repair or Escalate**

```yaml
steps:
  - ordinal: 1
    action: run_command
    command: "cargo test -p hkask-types -p hkask-cns -- --test-threads=1 2>&1 | tail -20"
    description: "Run fuzz tests"
    branching:
      success: 2
      failure: 5           # fuzz crashed → report failure

  - ordinal: 2
    action: classify
    classifier: qa-triage
    description: "Analyze fuzz failures from /tmp/fuzz.txt"
    branching:
      high_confidence: 3    # auto-repair
      medium_confidence: 4  # escalate
      low_confidence: 4
      flake: 6              # retry

  - ordinal: 3
    action: run_command
    command: "echo 'Auto-repair triggered'"
    description: "High-confidence auto-repair"
    cns_span: cns.qa.custom.auto_repair

  - ordinal: 4
    action: run_command
    command: "echo 'Escalate to human'"
    description: "Escalate medium/low confidence"

  - ordinal: 5
    action: run_command
    command: "echo 'Fuzz infrastructure failure'"
    description: "Report fuzz crash"

  - ordinal: 6
    action: loop
    command: "grep -c 'FAILED' /tmp/fuzz.txt || echo 0"
    description: "Retry fuzz up to 3 times for flakes"
    max_iterations: 3
    iteration_delay_secs: 10
    branching:
      success: 1            # back to fuzz
      failure: 4            # persistent, escalate
```

**Pattern: Contract Verification Loop**

```yaml
steps:
  - ordinal: 1
    action: run_command
    command: "cargo test -- --test-threads=1 2>&1"
    description: "Run contract tests"
    branching:
      success: 3
      failure: 2

  - ordinal: 2
    action: classify
    classifier: qa-triage
    description: "Classify contract test failures"
    branching:
      high_confidence: 4
      medium_confidence: 5
      low_confidence: 5

  - ordinal: 3
    action: run_command
    command: "echo 'All contracts hold'"
    description: "Success"

  - ordinal: 4
    action: run_command
    command: "echo 'Auto-fix contract violation'"
    description: "Repair confident violations"

  - ordinal: 5
    action: run_command
    command: "echo 'Flag for human contract review'"
    description: "Escalate ambiguous violations"
```

---

## 5. Classification Service

The classification service (`hkask-services-runtime`) is the decision engine for
all QA operations. It sends text passages to an LLM and returns structured
classifications.

### 5.1 Classifier Configs

Stored in `$HKASK_REPLICANT_REGISTRY_PATH/classify/` as YAML files. Two standard configs:

**`qa-triage.yaml`** — Used by classify steps in QA manifests:

```yaml
classifier:
  name: qa-triage
  model: google/gemma-4-26B-A4B-it
  provider: deepinfra
  system_prompt: |
    You are a QA triage classifier. Analyze the following Rust test failure.
    Output a JSON object with:
    - confidence (0.0-1.0): how confident you are in the diagnosis
    - is_flake (bool): true if this appears to be a non-deterministic/flaky failure
    - root_cause (string): brief description of the root cause
    - proposed_fix (string): suggested code fix, empty if uncertain
    - suggested_fuzz_target (string): fuzz target that would catch this, empty if n/a
  temperature: 0.1
  max_tokens: 512
```

**`qa-feedback.yaml`** — Used by the planned `kask qa suggest-fuzz` and correction passages:

### 5.2 Diagnosis Schema

The `ClassifyResponse` struct expected from the LLM:

```json
{
  "confidence": 0.96,
  "is_flake": false,
  "root_cause": "off-by-one error in index calculation at line 42",
  "proposed_fix": "Change `i <= len` to `i < len`",
  "failure_type": "panic"
}
```

The runner extracts `confidence` and `is_flake` for branching. The full response
is captured in the step output for CNS span enrichment.

---

## 6. CNS Integration

QA runs emit CNS spans automatically via `tracing`:

| Span | When Emitted |
|------|-------------|
| `cns.qa.repair_attempted` | Script start |
| `cns.qa.repair_verified` | Successful completion (PASS terminal) |
| `cns.qa.repair_exhausted` | Failure or error (FAIL/WARN/error) |

Spans include `manifest`, `terminal`, `steps`, `gas` fields. View via:

```bash
kask cns health
```

---

## 7. Test Harness Components

The `hkask-test-harness` crate provides reusable QA infrastructure:

| Module | Purpose |
|--------|---------|
| `test_runner` | `run_contract_tests()`, `inventory_contracts()`, `discover_uncontracted_functions()` |
| `fuzz` | Seed corpora for `cli_fuzz_seeds()` and `json_fuzz_seeds()` |

---

## 8. Recipes

### Run QA manifests via cargo test

```bash
# Run all QA integration tests
cargo test -p hkask-test-harness -- qa_script

# Run a specific manifest
cargo test -p hkask-test-harness -- qa_script::tests::run_comm_integration_gate
```

### Run contract tests

```bash
cargo run -- test
cargo run -- test -c hkask-keystore
```

### Create a custom autonomous script

1. Create `my-qa-script.yaml` following the [manifest format](#manifest-format)
2. Run it via cargo test:

```bash
cargo test -p hkask-test-harness -- qa_script
```

Or programmatically via the library:

```rust
use hkask_test_harness::qa_script;

let output = qa_script::run_script(
    Path::new("/workspace/root"),
    Path::new("registry/manifests/my-qa-script.yaml"),
).await?;
```

### Test a script manifest without LLM access

Manifests with `classifier_unavailable` branches gracefully degrade when
`DI_API_KEY` is not set — `run_command` steps execute normally, and
`classify` steps route to the unavailable branch.

### Integrate with CI

```bash
#!/bin/sh
# ci-qa.sh — QA gate for CI
set -e

echo "=== QA manifests ==="
cargo test -p hkask-test-harness -- qa_script::tests::run_

echo "=== Contract audit ==="
cargo run -- test
```

### Run the test harness tests

```bash
cargo test -p hkask-test-harness
```
