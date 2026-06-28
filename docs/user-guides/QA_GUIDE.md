# hKask QA System Guide

**Version:** v0.30.0  
**Last updated:** 2026-06-19  
**Audience:** Developers and operators of hKask

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [CLI Commands](#cli-commands)
   - [Fuzz Triage (`kask qa triage`)](#fuzz-triage)
   - [Mutation Analysis (`kask qa suggest-fuzz`)](#mutation-analysis)
   - [Autonomous Scripts (`kask qa run`)](#autonomous-scripts)
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

The hKask QA system provides **three layers** of automated quality assurance, from
passive analysis to fully autonomous interactive pipelines:

| Layer | Command | Input | Output | Autonomy |
|-------|---------|-------|--------|----------|
| **Fuzz Triage** | `kask qa triage` | bolero stderr | classified failures + auto-repair branches | Semi (classify + branch) |
| **Mutation Analysis** | `kask qa suggest-fuzz` | cargo-mutants output | fuzz target suggestions | Semi (classify + suggest) |
| **Autonomous Scripts** | `kask qa run --script <yaml>` | YAML manifest | script report with step outcomes | Full (classify → branch → loop) |

The system integrates with:

- **bolero** — coverage-guided fuzzer for Rust
- **cargo-mutants** — mutation testing framework
- **hkask-services-runtime** — LLM-powered classification (DeepInfra, OpenRouter, etc.)
- **CNS** — Cybernetic Nervous System for observability and homeostatic guards

### Principle Grounding

| Principle | How the QA system satisfies it |
|-----------|-------------------------------|
| **P4** Clear Boundaries | Every destructive action (auto-repair, PR creation) requires consent; gas budgets cap LLM costs |
| **P5** Essentialism | Three focused subcommands; one manifest format; no framework |
| **P8** Semantic Grounding | Every step emits a CNS span; classify results carry confidence scores |
| **P9** Homeostatic Self-Regulation | Autonomous branching adapts to classifier output; feedback loops improve future classifications |

---

## 2. Architecture

```
┌──────────────────────────────────────────────────────────────────────────┐
│  CLI Surface (hkask-cli)                                                  │
│                                                                           │
│  kask qa triage ──┐                                                      │
│  kask qa suggest  ─┤──> classify_batch() ──> LLM (DeepInfra/Gemma)       │
│  kask qa run      ─┘       ▲                                              │
│                            │                                              │
│  ┌─────────────────────────┴──────────────────────────────────┐         │
│  │  Classification Service (hkask-services-runtime)            │         │
│  │  - classify_batch()   → batch LLM classification            │         │
│  │  - extract_triples()  → structured data extraction          │         │
│  │  - ClassifierConfig   → loaded from registry YAML           │         │
│  └─────────────────────────┬──────────────────────────────────┘         │
│                            │                                              │
│  ┌─────────────────────────┴──────────────────────────────────┐         │
│  │  Test Harness (hkask-test-harness)                          │         │
│  │  - triage/              → bolero parsing + auto-repair       │         │
│  │  - feedback/            → correction passages + mutants      │         │
│  │  - qa_script/           → QaScriptRunner + manifest types    │         │
│  │  - prob_contract/       → PAC probabilistic verification    │         │
│  │  - strategies/          → proptest generators                │         │
│  │  - test_runner/         → contract test discovery + runner   │         │
│  └────────────────────────────────────────────────────────────┘         │
│                                                                           │
│  ┌──────────────────────────────────────────────────────────┐           │
│  │  CNS (hkask-cns)                                           │           │
│  │  - cns.qa.bolero_failure   → fuzz failure detected         │           │
│  │  - cns.qa.autonomous.*     → script step outcomes          │           │
│  │  - cns.classify            → classification operations     │           │
│  └──────────────────────────────────────────────────────────┘           │
└──────────────────────────────────────────────────────────────────────────┘
```

### Data Flow: Autonomous Script

```
YAML Manifest ──> QaScriptManifest (parse)
                      │
                      ▼
              QaScriptRunner::run()
                      │
         ┌────────────┼────────────┐
         ▼            ▼            ▼
    run_command   classify      loop
         │            │            │
         │    ┌───────┴───────┐    │
         │    ▼               ▼    │
         │  high_conf   medium_conf│
         │    │               │    │
         ▼    ▼               ▼    ▼
      branch to specified ordinal (or advance linearly)
                      │
                      ▼
              QaScriptReport
```

---

## 3. CLI Commands

### 3.1 Fuzz Triage

Classify bolero fuzz failures and optionally auto-repair high-confidence issues.

```bash
# Pipe bolero output directly
cargo bolero test --all-features 2>&1 | kask qa triage

# From a saved file
kask qa triage --input fuzz-output.txt
```

**What it does:**

1. Parses bolero stderr into structured `BoleroFailure` records (crate, test, panic, stack trace)
2. Loads the `qa-triage` classifier config from `$HKASK_REGISTRY_PATH/classify/qa-triage.yaml`
3. Sends each failure as a passage to `classify_batch()` via DeepInfra
4. Routes by confidence:
   - **≥0.95** — High confidence → auto-repair branch created
   - **0.70–0.95** — Medium confidence → issue opened
   - **<0.70** — Low confidence → investigation needed
   - **is_flake** — Flake detected → skipped
5. Emits `cns.qa.bolero_failure` spans for observability

**Requirements:**
- `DI_API_KEY` environment variable (or equivalent provider key)
- Classifier config at `registry/classify/qa-triage.yaml`
- `cargo bolero` installed

**Without LLM:** Falls back to parse-only mode — prints failures without classification.

### 3.2 Mutation Analysis

Suggest fuzz targets for surviving mutants from `cargo-mutants`.

```bash
cargo mutants --all-features 2>&1 | kask qa suggest-fuzz
```

**What it does:**

1. Parses `cargo-mutants` output for "Uncaught mutants in crate: file:line"
2. Loads the `qa-feedback` classifier config
3. Formats each surviving mutant as a passage describing the mutation
4. Requests fuzz target suggestions from the LLM
5. Prints crate/file/line → suggested fuzz target

**Requirements:**
- `DI_API_KEY` environment variable
- Classifier config at `registry/classify/qa-feedback.yaml`
- `cargo mutants` installed

### 3.3 Autonomous Scripts

Execute a YAML-defined interactive QA pipeline with classifier-driven branching.

```bash
kask qa run --script registry/manifests/qa-autonomous.yaml
```

**What it does:**

1. Parses the YAML manifest into a `QaScriptManifest`
2. Wires the classification service into a `QaScriptRunner`
3. Executes steps in order, branching on classifier outcomes or command exit codes
4. Loop steps retry with configurable backoff
5. Produces a structured report with per-step outcomes

**Step-by-step output example:**

```
[QA] Loading script: registry/manifests/qa-autonomous.yaml
[QA] Running script 'qa-autonomous' — 7 steps
[QA] ──────────────────────────────────────────────
[QA] ──────────────────────────────────────────────
[QA] Script complete: 3 steps executed, terminal outcome: high_confidence
  [1] run_command → success (1234ms)
  [2] classify → high_confidence (567ms) | category: {"confidence":0.96,"is_flake":false,...}
  [3] run_command → success (12ms)
```

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
    command: "cargo bolero test --timeout 30s 2>&1 | tee /tmp/fuzz.txt"
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

Stored in `$HKASK_REGISTRY_PATH/classify/` as YAML files. Two standard configs:

**`qa-triage.yaml`** — Used by `kask qa triage` and classify steps:

```yaml
classifier:
  name: qa-triage
  model: google/gemma-4-9b-it
  provider: deepinfra
  system_prompt: |
    You are a QA triage classifier. Analyze the following Rust fuzz test failure.
    Output a JSON object with:
    - confidence (0.0-1.0): how confident you are in the diagnosis
    - is_flake (bool): true if this appears to be a non-deterministic/flaky failure
    - root_cause (string): brief description of the root cause
    - proposed_fix (string): suggested code fix, empty if uncertain
    - suggested_fuzz_target (string): fuzz target that would catch this, empty if n/a
  temperature: 0.1
  max_tokens: 512
  fallback_category: '{"confidence":0.0,"is_flake":false,"root_cause":"unparseable"}'
```

**`qa-feedback.yaml`** — Used by `kask qa suggest-fuzz` and correction passages:

```yaml
classifier:
  name: qa-feedback
  model: google/gemma-4-9b-it
  provider: deepinfra
  system_prompt: |
    You are a QA feedback classifier. Given a surviving mutant from mutation testing,
    suggest a fuzz target that would catch it. Output the suggestion as plain text.
  temperature: 0.3
  max_tokens: 256
  fallback_category: "No suggestion"
```

### 5.2 Diagnosis Schema

The `QaDiagnosis` struct expected from the LLM:

```json
{
  "confidence": 0.96,
  "is_flake": false,
  "root_cause": "off-by-one error in index calculation at line 42",
  "proposed_fix": "Change `i <= len` to `i < len`",
  "suggested_fuzz_target": "fuzz_index_bounds",
  "affected_file": "src/parser.rs",
  "affected_line": 42,
  "failure_type": "panic"
}
```

The runner extracts `confidence` and `is_flake` for branching. The full diagnosis
is used by `kask qa triage` for auto-repair and CNS span enrichment.

---

## 6. CNS Integration

Every QA operation emits CNS spans for observability and homeostatic monitoring:

| Span Target | When Emitted | Fields |
|-------------|-------------|--------|
| `cns.qa.bolero_failure` | Each fuzz failure parsed (parse-only mode) | `crate_name`, `test_name` |
| `cns.qa.script` | Each autonomous script step | `manifest`, `ordinal`, `action`, `outcome`, `duration_ms` |
| `cns.classify` | Each classification batch | `operation`, `item_count` |

Custom manifests can set per-step `cns_span` targets:

```yaml
steps:
  - ordinal: 3
    action: run_command
    cns_span: cns.qa.custom.auto_repair   # Custom span target
```

View CNS health:

```bash
kask cns health
```

---

## 7. Test Harness Components

The `hkask-test-harness` crate provides reusable QA infrastructure:

| Module | Purpose |
|--------|---------|
| `triage` | `BoleroFailure` parsing, `QaDiagnosis`, `TriageReport`, auto-repair (`attempt_auto_repair`, `open_pull_request`) |
| `feedback` | Correction passages for rejected repairs, mutant-to-fuzz suggestion formatting |
| `qa_script` | `QaScriptManifest`, `QaScriptRunner`, `QaScriptReport` |
| `prob_contract` | `ProbContractRunner` for PAC-style (p, δ, k)-satisfaction testing |
| `strategies` | `proptest` strategy functions for core types (`any_nu_event`, `any_triple`, etc.) |
| `test_runner` | `run_contract_tests()`, `inventory_contracts()`, `discover_uncontracted_functions()` |
| `fuzz` | Seed corpora for `cli_fuzz_seeds()` and `json_fuzz_seeds()` |

---

## 8. Recipes

### Run a full autonomous QA cycle

```bash
kask qa run --script registry/manifests/qa-autonomous.yaml
```

### Triage fuzz failures with LLM classification

```bash
cargo bolero test --all-features 2>&1 | kask qa triage
```

### Discover uncontracted functions and suggest fuzz targets

```bash
# 1. Find functions without REQ contracts
kask test audit

# 2. Run mutation testing
cargo mutants --all-features 2>&1 | kask qa suggest-fuzz
```

### Create a custom autonomous script

1. Create `my-qa-script.yaml` following the [manifest format](#manifest-format)
2. Optionally create a custom classifier config in `registry/classify/my-classifier.yaml`
3. Run it:

```bash
kask qa run --script my-qa-script.yaml
```

### Test a script manifest without LLM access

The runner still executes `run_command` steps without a classifier — useful for
testing the flow before wiring in expensive LLM calls:

```yaml
steps:
  - ordinal: 1
    action: run_command
    command: "echo 'test pass'"
    branching:
      success: 2
  - ordinal: 2
    action: run_command
    command: "echo 'all done'"
```

```bash
kask qa run --script dry-run.yaml
# Output:
# [QA] Script complete: 2 steps executed, terminal outcome: success
#   [1] run_command → success (5ms)
#   [2] run_command → success (4ms)
```

### Integrate with CI

```bash
#!/bin/sh
# ci-qa.sh — Autonomous QA gate for CI
set -e

echo "=== Fuzz tests ==="
cargo bolero test --all-features --timeout 60s 2>&1 | tee ci-fuzz.txt
kask qa triage --input ci-fuzz.txt

echo "=== Autonomous QA script ==="
kask qa run --script registry/manifests/qa-autonomous.yaml

echo "=== Contract audit ==="
kask test audit 2>&1 | tee ci-contract.txt
```

### Run the test harness tests

```bash
cargo test -p hkask-test-harness
```
