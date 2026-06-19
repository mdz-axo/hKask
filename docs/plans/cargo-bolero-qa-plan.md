---
title: "cargo-bolero QA Plan — Lightweight Autonomous Testing for hKask"
audience: [engineers, agents, replicants]
last_updated: 2026-06-18
version: "0.28.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle]
---

# cargo-bolero QA Plan

**Principle:** Zero production-code annotations. All QA infrastructure lives in `fuzz/`, `tests/`, CI scripts, and one classifier config. If any piece becomes unmanageable, delete it — the system still works.

**Classifier model:** `google/gemma-4-26B-A4B-it` (3.8B active, Apache 2.0, already deployed via DeepInfra in `hkask-services-classify`).

**Foundation:** `cargo-bolero` (unified fuzz + property testing + formal verification front-end) + `cargo-mutants` (mutation testing).

**Process:** Toyota Kata — 4-step scientific pattern. Navigate with a compass, not a map.

---

## Kata Step 1: Understand the Direction

**Challenge:** hKask has zero automated regression detection. A panic in `hkask-cns` or `hkask-types` cascades through every downstream crate with no automated signal until a human notices. The prior contract system was removed because it suffocated the code with annotations. The QA system must find real bugs without annotating production code.

**Capability gap:** No fuzzing, no mutation testing, no automated failure triage. The system is blind to regressions.

---

## Kata Step 2: Grasp the Current Condition

Before any implementation, run this audit. Facts only — no interpretation.

```bash
# Test count: how many tests exist today?
grep -rn '#\[test\]' crates/ --include='*.rs' | wc -l
grep -rn '#\[tokio::test\]' crates/ --include='*.rs' | wc -l

# Unsafe surface: highest fuzzing priority
grep -rn 'unsafe' crates/ --include='*.rs' | wc -l

# Public function count: scale of the testing surface
grep -rn 'pub fn' crates/ --include='*.rs' | grep -v '/tests/' | wc -l
grep -rn 'pub async fn' crates/ --include='*.rs' | grep -v '/tests/' | wc -l

# Existing CNS error patterns: what's already failing?
# (CNS spans that indicate bugs, not intentional alerts)

# Dependency: can we install cargo-bolero?
cargo install cargo-bolero --dry-run 2>&1
# (requires binutils-dev, libunwind-dev on Debian/Ubuntu)
```

**Record these numbers in the Kata board before proceeding.** They are the baseline. If test count is zero (as earlier grep suggests), that's the first obstacle — not fuzzing, not mutation testing, not LLM triage. You can't measure mutation score when there are no tests to catch mutants.

---

## Kata Step 3: Establish the Next Target Condition

**Target (achieve by: 1 week):**

| Metric | Current | Target |
|--------|---------|--------|
| `#[test]` count across all crates | TBD (run audit) | ≥ 15 (3–5 per P0 crate) |
| Fuzz targets on substrate crates | 0 | ≥ 3 (hkask-types, hkask-cns, hkask-inference) |
| `cargo bolero test --all` exits clean | N/A | Yes |
| `cargo mutants` score on hkask-cns | 0% | > 0% |
| `kask qa triage` classifies a failure | N/A | CNS span `cns.qa.bolero_failure` emitted |

**Target (achieve by: 3 weeks):**

| Metric | Target |
|--------|--------|
| Fuzz targets on all P1 crates | ≥ 6 (services-core, storage, memory, etc.) |
| Auto-repair PR opened for a real bolero failure | ≥ 1 |
| Feedback loop: rejected repair feeds back to classifier | 1 complete cycle |
| `cargo mutants` score on P0 crates | > 25% |

**Obstacles Parking Lot** (visible now; address one at a time during Step 4):

1. Zero existing tests → mutation score is meaningless without a baseline
2. bolero requires system libraries on Linux (binutils-dev, libunwind-dev)
3. bolero failure output format unknown — needs reverse-engineering
4. Classifier may return malformed JSON → needs fallback path
5. `git apply` can fail on stale diffs → needs check + rollback
6. Auto-repair PRs can duplicate → needs dedup logic
7. `cargo mutants --in-place` corrupts working tree if CI is killed
8. Surviving mutants are wasted signal if not fed back to the system

---

## Kata Step 4: Iterate Toward the Target Condition

Work one obstacle at a time. Plan → Do → Check → Act.

---

### Obstacle 1: Zero existing tests

**Plan:** Before fuzzing or mutation testing, add 3–5 basic unit tests to each P0 substrate crate. Happy-path only. No contracts, no property testing, no DSL. Just `assert_eq!`.

**Do:**

```rust
// crates/hkask-types/tests/smoke.rs
#[test]
fn cns_health_constructs() {
    let health = hkask_types::cns::CnsHealth {
        overall_deficit: 0,
        critical_count: 0,
        warning_count: 0,
        healthy: true,
    };
    assert!(health.healthy);
}

#[test]
fn cns_span_display_roundtrips() {
    use std::str::FromStr;
    let span = hkask_types::cns::CnsSpan::Inference;
    let s = span.to_string();
    let parsed = hkask_types::cns::CnsSpan::from_str(&s).unwrap();
    assert_eq!(parsed, span);
}

#[test]
fn queue_depth_never_negative() {
    let qd = hkask_types::cns::QueueDepth::new(-5.0);
    assert!(qd.as_raw() >= 0.0);
}
```

**Check:** `cargo test -p hkask-types` passes. Now mutation testing has something to measure.

**Act:** If ≥ 3 tests pass per P0 crate, proceed to Obstacle 2. If any crate's API is too coupled to write a simple test, that's architectural feedback — record it and move on.

---

### Obstacle 2: No fuzzing infrastructure

**Plan:** Add `bolero` dependency. Write fuzz targets prioritized by bug density potential. Start with the highest-yield surfaces.

**Fuzz target priority heuristic:**

| Priority | Surface | Rationale |
|----------|---------|-----------|
| 1 | Every `pub fn` containing `unsafe` | Highest bug density |
| 2 | Every parser/deserializer (`FromStr`, `Deserialize`, `from_bytes`) | Input boundary — where malformed data enters |
| 3 | Every function with `Vec`, `HashMap`, or indexing (`[]`, `.get()`) | Panic surface |
| 4 | Every function with arithmetic operations | Silent overflow/corruption |
| 5 | Everything else | Diminishing returns |

**Do:**

Add to workspace `Cargo.toml`:

```toml
[workspace.dependencies]
bolero = "0.13"
```

Install subcommand:

```bash
# Requires binutils-dev, libunwind-dev on Debian/Ubuntu
sudo apt install binutils-dev libunwind-dev
cargo install cargo-bolero
```

Write P0 fuzz targets:

```rust
// crates/hkask-types/fuzz/fuzz_targets/types_fuzz.rs
#![no_main]
use bolero::check;

#[test]
fn fuzz_queue_depth_construct() {
    check!()
        .with_type::<f64>()
        .for_each(|v| {
            let qd = hkask_types::cns::QueueDepth::new(*v);
            assert!(qd.as_raw() >= 0.0); // invariant: never negative
        });
}

#[test]
fn fuzz_cns_health_fields() {
    check!()
        .with_type::<(u64, usize, usize, bool)>()
        .for_each(|(deficit, critical, warning, healthy)| {
            let health = hkask_types::cns::CnsHealth {
                overall_deficit: *deficit,
                critical_count: *critical,
                warning_count: *warning,
                healthy: *healthy,
            };
            // Coherence: zero deficit should imply healthy
            if health.overall_deficit == 0 && health.critical_count == 0 {
                assert!(health.healthy);
            }
        });
}
```

```rust
// crates/hkask-cns/fuzz/fuzz_targets/cns_fuzz.rs
#![no_main]
use bolero::check;

#[test]
fn fuzz_cns_span_parse() {
    check!()
        .with_type::<String>()
        .for_each(|s| {
            // CNS span parsing must never panic on arbitrary input
            let _ = s.parse::<hkask_types::cns::CnsSpan>();
        });
}
```

```rust
// crates/hkask-inference/fuzz/fuzz_targets/inference_fuzz.rs
#![no_main]
use bolero::check;

#[test]
fn fuzz_model_name_routing() {
    check!()
        .with_type::<String>()
        .for_each(|model_name| {
            // Model name parsing must never panic
            // (tests the XX/ prefix parsing in InferenceRouter)
            let _ = model_name.find('/');
        });
}
```

**Check:**

```bash
# Fast mode (CI): property testing, ~10s
cargo bolero test --all

# Thorough mode: coverage-guided fuzzing, 60s
cargo bolero test --all --engine libfuzzer --duration 60

# Formal verification (selected targets only):
cargo bolero test --engine kani fuzz_queue_depth_construct
```

**Act:** If any fuzz target finds a panic, that's a real bug — fix it before proceeding. If no panics found in 60s of libFuzzer, proceed.

---

### Obstacle 3: No mutation testing baseline

**Plan:** Add `cargo-mutants`. Run on P0 crates. Record score.

**Do:**

```bash
# Use WITHOUT --in-place: copies to temp dir, safer in CI
cargo mutants --timeout 60
```

**Note on `--in-place`:** The original plan used `--in-place` for speed. Don't. If CI is killed mid-mutation, the working tree is corrupted. Use the default temp-dir mode. The speed difference is negligible for a 60-second run.

**Check:** Mutation score > 0% on at least hkask-types. Any killed mutant proves the smoke tests catch something.

**Act:** Record the score. This is the baseline. Surviving mutants become input for Obstacle 7 (feedback loop).

---

### Obstacle 4: Classifier integration

**Plan:** Add a QA classifier config to `registry/classify/`. Compress the system prompt — move confidence guidelines to Rust post-processing, leave only the JSON schema in the prompt.

**Do:**

```yaml
# registry/classify/qa-triage.yaml
classifier:
  name: qa-triage
  model: google/gemma-4-26B-A4B-it
  provider: deepinfra
  concurrency: 10
  timeout_secs: 30

  system_prompt: >
    Diagnose this Rust test failure. Return ONLY:
    {"failure_type":"Panic|Assertion|Timeout|Flake|LogicError|MemoryError",
     "root_cause":"one sentence","confidence":0.0-1.0,
     "proposed_fix":"unified diff or empty",
     "affected_file":"relative path","affected_line":0,
     "is_flake":false,"suggested_fuzz_target":"description"}

  base_url: https://api.deepinfra.com/v1/openai/chat/completions
  api_key_env: DEEPINFRA_API_KEY

  temperature: 0.0
  max_tokens: 500

  fallback_category: Unknown
```

The confidence guidelines (0.95+ → auto-PR, 0.70–0.94 → issue with suggestion, <0.70 → investigation) are enforced in Rust — the LLM just returns a number.

**Check:** `load_classifier_config("qa-triage")` resolves correctly.

---

### Obstacle 5: CNS QA spans

**Plan:** Add only the spans that survive the deletion test. Five, not seven.

**Do:**

```rust
// crates/hkask-types/src/cns.rs — add to CnsSpan enum
pub enum CnsSpan {
    // ... existing variants ...

    /// A cargo-bolero fuzz target caught a failure.
    QaBoleroFailure,
    /// An autonomous repair was attempted.
    QaRepairAttempted,
    /// A repair passed verification (all tests green).
    QaRepairVerified,
    /// Repairs exhausted — human investigation needed.
    QaRepairExhausted,
    /// A mutant survived — test suite has a gap.
    QaMutantSurvived,
    /// A QA metric was recorded.
    QaMetric {
        name: &'static str,
        value: f64,
    },
}
```

**Deletion test results:**

| Removed | Why |
|---------|-----|
| `QaTriageComplete` | Redundant — `cns.classify` already logs classification completion on `cns.classify` target |
| `QaMutationRunComplete { score, killed, survived }` | Folded into `QaMetric { name: "mutation_score", value: 0.67 }` — one generic span replaces multiple specific ones |

**Check:** `cargo check -p hkask-types` passes with new variants.

---

### Obstacle 6: Triage pipeline (CI + CLI integration)

**Plan:** Don't build a standalone binary. Add `kask qa triage` as a CLI subcommand that reads bolero output from stdin (piped from CI). The existing `hkask-services-classify::classify_batch` handles the LLM call. The routing logic (confidence gates, git operations, PR/issue creation) lives in `hkask-test-harness` as a library.

**Architecture:**

```
CI: cargo bolero test --all 2>&1 | kask qa triage
                                   │
                                   ▼
                          hkask-test-harness (lib)
                          ├── parse bolero output
                          ├── format passages
                          ├── classify_batch (existing)
                          ├── route by confidence
                          ├── git: check --apply + rollback
                          ├── dedup: check existing branches/PRs
                          └── open PR or issue
```

**Why CLI subcommand over standalone binary:**
- Reuses existing CLI infrastructure (`kask` already exists)
- Reuses existing CNS integration
- Discoverable via `kask help`
- No new binary to compile in CI

**Do:**

```rust
// crates/hkask-test-harness/src/triage.rs (library)
// Called by kask CLI subcommand: `kask qa triage`

use hkask_services_classify::{classify_batch, load_classifier_config};
use serde::Deserialize;
use std::io::{self, BufRead};
use std::process::Command;

#[derive(Debug, Deserialize)]
struct QaDiagnosis {
    failure_type: String,
    root_cause: String,
    confidence: f64,
    #[serde(default)]
    proposed_fix: String,
    #[serde(default)]
    affected_file: String,
    #[serde(default)]
    affected_line: u32,
    #[serde(default)]
    is_flake: bool,
    #[serde(default)]
    suggested_fuzz_target: String,
}

struct BoleroFailure {
    crate_name: String,
    test_name: String,
    panic_message: String,
    stack_trace: String,
    source_snippet: String,
    failing_input: String,
}

pub async fn triage(stdin: impl BufRead) -> Result<TriageReport, TriageError> {
    let failures = parse_bolero_stdin(stdin)?;

    if failures.is_empty() {
        tracing::info!(target: "cns.qa", "No bolero failures to triage");
        return Ok(TriageReport::empty());
    }

    tracing::info!(target: "cns.qa", failure_count = failures.len(), "Triaging");

    let config = load_classifier_config("qa-triage")?;
    let passages: Vec<String> = failures.iter().map(|f| f.to_passage()).collect();
    let results = classify_batch(&passages, config).await?;

    let mut report = TriageReport::new();

    for (i, result) in results.iter().enumerate() {
        let diagnosis = match serde_json::from_str::<QaDiagnosis>(&result.category) {
            Ok(d) => d,
            Err(e) => {
                // Parser failure: open human issue with raw output attached.
                // Don't silently drop — a real bolero failure needs attention.
                tracing::warn!(target: "cns.qa", index = i, error = %e,
                    "Classifier returned unparseable JSON — opening human issue");
                open_raw_failure_issue(&failures[i], &result.category)?;
                report.unparseable += 1;
                continue;
            }
        };

        emit_cns_span(&failures[i], &diagnosis);

        if diagnosis.is_flake {
            report.flakes += 1;
            continue;
        }

        // Confidence routing with dedup guard
        match diagnosis.confidence {
            c if c >= 0.95 => {
                if already_has_repair_branch(&failures[i])? {
                    tracing::info!(target: "cns.qa", "Duplicate repair blocked");
                    report.duplicates_blocked += 1;
                } else {
                    attempt_auto_repair(&failures[i], &diagnosis).await?;
                    report.auto_repaired += 1;
                }
            }
            c if c >= 0.70 => {
                open_issue_with_suggestion(&failures[i], &diagnosis)?;
                report.issues_opened += 1;
            }
            _ => {
                open_issue_for_investigation(&failures[i], &diagnosis)?;
                report.issues_opened += 1;
            }
        }
    }

    Ok(report)
}

fn parse_bolero_stdin(stdin: impl BufRead) -> Result<Vec<BoleroFailure>, TriageError> {
    // bolero outputs failures in a structured format to stdout.
    // Parse: crate name, test name, panic message, location, input.
    // This needs reverse-engineering from bolero's actual output.
    // For now: read lines, detect "Test Failure" separator blocks.
    let mut failures = Vec::new();
    let mut current: Option<BoleroFailureBuilder> = None;

    for line in stdin.lines() {
        let line = line?;
        if line.contains("Test Failure") {
            if let Some(builder) = current.take() {
                if let Ok(f) = builder.build() {
                    failures.push(f);
                }
            }
            current = Some(BoleroFailureBuilder::new());
        } else if let Some(ref mut b) = current {
            b.feed(&line);
        }
    }
    if let Some(builder) = current.take() {
        if let Ok(f) = builder.build() {
            failures.push(f);
        }
    }

    Ok(failures)
}

fn already_has_repair_branch(failure: &BoleroFailure) -> Result<bool, TriageError> {
    let branch = format!("auto-heal/{}", slugify(&failure.test_name));
    let output = Command::new("git")
        .args(["branch", "--list", &branch])
        .output()?;
    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

async fn attempt_auto_repair(
    failure: &BoleroFailure,
    diagnosis: &QaDiagnosis,
) -> Result<(), TriageError> {
    let branch = format!("auto-heal/{}", slugify(&failure.test_name));

    tracing::info!(target: "cns.qa.repair_attempted",
        crate_name = %failure.crate_name, confidence = diagnosis.confidence);

    // 1. Create branch
    run_git(&["checkout", "-b", &branch])?;

    // 2. Check that the diff applies cleanly BEFORE applying
    let check = Command::new("git")
        .args(["apply", "--check"])
        .stdin(std::process::Stdio::piped())
        .spawn()?;
    // Write diff to stdin...

    if !check.wait()?.success() {
        tracing::warn!(target: "cns.qa", "Diff does not apply cleanly — rolling back");
        run_git(&["checkout", "--", "."])?;
        run_git(&["checkout", "-"])?;  // back to previous branch
        run_git(&["branch", "-D", &branch])?;
        return Ok(());
    }

    // 3. Apply the fix
    run_git_with_stdin(&["apply"], &diagnosis.proposed_fix)?;

    // 4. Verify
    let test_ok = Command::new("cargo")
        .args(["bolero", "test", "--all"])
        .status()?
        .success();

    if !test_ok {
        tracing::warn!(target: "cns.qa", "Verification failed — rolling back");
        run_git(&["checkout", "--", "."])?;
        run_git(&["checkout", "-"])?;
        run_git(&["branch", "-D", &branch])?;
        return Ok(());
    }

    let mutation_result = Command::new("cargo")
        .args(["mutants", "--timeout", "60"])
        .output()?;

    // 5. Push and open PR
    run_git(&["push", "-u", "origin", &branch])?;
    open_pull_request(failure, diagnosis, &branch)?;

    tracing::info!(target: "cns.qa.repair_verified", "Repair verified and PR opened");
    Ok(())
}

fn run_git(args: &[&str]) -> Result<(), TriageError> {
    let status = Command::new("git").args(args).status()?;
    if !status.success() {
        return Err(TriageError::Git(format!("git {} failed", args.join(" "))));
    }
    Ok(())
}

fn run_git_with_stdin(args: &[&str], stdin_text: &str) -> Result<(), TriageError> {
    let mut child = Command::new("git")
        .args(args)
        .stdin(std::process::Stdio::piped())
        .spawn()?;
    use std::io::Write;
    child.stdin.take().unwrap().write_all(stdin_text.as_bytes())?;
    if !child.wait()?.success() {
        return Err(TriageError::Git(format!("git {} failed", args.join(" "))));
    }
    Ok(())
}

fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect()
}

// ── Stubs for GitHub integration ──────────────────────────

fn open_pull_request(
    _failure: &BoleroFailure,
    _diagnosis: &QaDiagnosis,
    _branch: &str,
) -> Result<(), TriageError> {
    // Implement with octocrab or gh CLI
    Ok(())
}

fn open_issue_with_suggestion(
    _failure: &BoleroFailure,
    _diagnosis: &QaDiagnosis,
) -> Result<(), TriageError> {
    Ok(())
}

fn open_issue_for_investigation(
    _failure: &BoleroFailure,
    _diagnosis: &QaDiagnosis,
) -> Result<(), TriageError> {
    Ok(())
}

fn open_raw_failure_issue(
    _failure: &BoleroFailure,
    _raw_output: &str,
) -> Result<(), TriageError> {
    // Classifier returned unparseable JSON — open issue with raw bolero output
    Ok(())
}

fn emit_cns_span(failure: &BoleroFailure, diagnosis: &QaDiagnosis) {
    tracing::info!(
        target: "cns.qa.bolero_failure",
        crate_name = %failure.crate_name,
        test_name = %failure.test_name,
        failure_type = %diagnosis.failure_type,
        confidence = diagnosis.confidence,
        is_flake = diagnosis.is_flake,
    );
}

#[derive(Debug, Default)]
pub struct TriageReport {
    pub auto_repaired: usize,
    pub issues_opened: usize,
    pub flakes: usize,
    pub unparseable: usize,
    pub duplicates_blocked: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum TriageError {
    #[error("classifier error: {0}")]
    Classifier(#[from] hkask_services_core::ServiceError),
    #[error("git error: {0}")]
    Git(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ── Bolero output parsing (reverse-engineer from actual output) ──

struct BoleroFailureBuilder {
    // Accumulate fields from bolero's structured output
}

impl BoleroFailureBuilder {
    fn new() -> Self { Self {} }
    fn feed(&mut self, _line: &str) {}
    fn build(self) -> Result<BoleroFailure, TriageError> {
        todo!("Reverse-engineer from bolero's actual failure output format")
    }
}

impl BoleroFailure {
    fn to_passage(&self) -> String {
        format!(
            "CRATE: {crate}\nTEST: {test}\nPANIC: {panic}\nINPUT: {input}\nSTACK:\n{stack}\nSOURCE:\n{source}",
            crate = self.crate_name,
            test = self.test_name,
            panic = self.panic_message,
            input = self.failing_input,
            stack = self.stack_trace,
            source = self.source_snippet,
        )
    }
}
```

**Check:** Pipe a known bolero failure through `kask qa triage`. Verify CNS spans emitted. Verify no duplicate branches created. Verify unparseable JSON opens a human issue.

---

### Obstacle 7: Feedback loop (close the PDCA cycle)

**Plan:** Two feedback paths to make the system improve over time.

**Path A — Human-rejected repairs feed back to classifier.**

When a human rejects an auto-repair PR, capture the rejection reason and the corrected fix. Format as a "correction passage" and feed back through `classify_batch` with a `qa-feedback` classifier config (same model, different system prompt: "You previously diagnosed this failure as X. The correct diagnosis was Y. Learn from this discrepancy."). This improves future classifications without fine-tuning — it's in-context learning via the passage.

**Path B — Surviving mutants suggest new fuzz targets.**

When `cargo mutants` reports uncaught mutants, format each surviving mutant's location and mutation as a passage. Feed through the classifier: "This mutant survived at health.rs:42 (changed > to >=). Suggest a fuzz target that would catch it." The suggested fuzz target gets appended to the crate's fuzz file.

```rust
// crates/hkask-test-harness/src/feedback.rs

pub async fn feed_surviving_mutants(mutants: &[SurvivingMutant]) -> Result<Vec<FuzzSuggestion>, TriageError> {
    let passages: Vec<String> = mutants.iter().map(|m| {
        format!(
            "CRATE: {}\nFILE: {}\nLINE: {}\nMUTATION: changed {} to {}\n\
             Suggest a fuzz target that would catch this mutant.",
            m.crate_name, m.file, m.line, m.original, m.mutated
        )
    }).collect();

    let config = load_classifier_config("qa-feedback")?;
    let results = classify_batch(&passages, config).await?;
    // Parse suggestions, write to crate's fuzz file
    todo!("Parse and append fuzz target suggestions")
}

pub async fn feed_rejected_repair(
    original_failure: &BoleroFailure,
    incorrect_diagnosis: &QaDiagnosis,
    correct_fix: &str,
) -> Result<(), TriageError> {
    let passage = format!(
        "CORRECTION:\n\
         Original failure: {failure}\n\
         You diagnosed: {incorrect}\n\
         Correct diagnosis: {correct}\n\
         Learn from this discrepancy.",
        failure = original_failure.to_passage(),
        incorrect = incorrect_diagnosis.root_cause,
        correct = correct_fix,
    );
    let config = load_classifier_config("qa-feedback")?;
    classify_batch(&[passage], config).await?;
    Ok(())
}
```

**Check:** After one rejected PR, verify `qa-feedback` classifier processes the correction passage. After one mutation run, verify at least one suggested fuzz target is generated.

---

## Architecture Diagram (Revised)

```
┌─────────────────────────────────────────────────────────────────────┐
│                    hKask Lightweight QA Stack                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────┐   ┌───────────────┐   ┌──────────────┐               │
│  │  SMOKE   │   │ cargo-bolero  │   │ cargo-mutants│               │
│  │  TESTS   │   │               │   │              │               │
│  │          │   │ Fuzz targets  │   │ Zero config  │               │
│  │ 3-5 per  │   │ prioritized:  │   │ cargo sub-   │               │
│  │ P0 crate │   │ unsafe >     │   │ command      │               │
│  │          │   │ parsers >     │   │ (temp dir,   │               │
│  │ Baseline │   │ indexing >    │   │ not --in-    │               │
│  │ for mut  │   │ arithmetic    │   │ place)       │               │
│  │ scoring  │   │               │   │              │               │
│  └────┬─────┘   └──────┬────────┘   └──────┬───────┘               │
│       │                 │                   │                        │
│       │    ┌────────────┘                   │                        │
│       │    ▼                                ▼                        │
│       │  ┌──────────────────────────────────────────┐               │
│       │  │         kask qa triage (CLI)              │               │
│       │  │                                          │               │
│       │  │  Reads bolero output from stdin           │               │
│       │  │  → classify_batch (Gemma 4 26B)          │               │
│       │  │  → routes by confidence:                  │               │
│       │  │    ≥ 0.95: auto-PR (with dedup guard)     │               │
│       │  │    0.70-0.94: issue with suggestion       │               │
│       │  │    < 0.70: issue for investigation        │               │
│       │  │    unparseable: issue with raw output      │               │
│       │  │  → git apply --check before applying      │               │
│       │  │  → rollback on verification failure       │               │
│       │  └────────────────┬─────────────────────────┘               │
│       │                   │                                          │
│       │    ┌──────────────┴──────────────┐                          │
│       │    ▼                              ▼                          │
│       │  ┌──────────────────┐   ┌────────────────────┐              │
│       │  │  FEEDBACK LOOP   │   │  FEEDBACK LOOP     │              │
│       │  │                  │   │                    │              │
│       │  │ Rejected PRs →   │   │ Surviving mutants  │              │
│       │  │ correction       │   │ → fuzz target      │              │
│       │  │ passage →        │   │ suggestions →      │              │
│       │  │ qa-feedback      │   │ append to fuzz/*   │              │
│       │  │ classifier       │   │                    │              │
│       │  └──────────────────┘   └────────────────────┘              │
│       │                                                              │
│       ▼                                                              │
│  ┌────────────────────────────────────────────────────────────┐     │
│  │                    CNS QA Spans (5)                         │     │
│  │  cns.qa.bolero_failure  cns.qa.repair_attempted            │     │
│  │  cns.qa.repair_verified cns.qa.repair_exhausted            │     │
│  │  cns.qa.mutant_survived cns.qa.metric {name, value}        │     │
│  └────────────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## CI Integration

```yaml
# .github/workflows/qa.yml
name: QA

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  schedule:
    - cron: '0 4 * * *'  # nightly deep QA

jobs:
  # ── Fast path: every push, every PR ──────────────────────────────
  quick-qa:
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Install system deps for bolero
        run: sudo apt-get install -y binutils-dev libunwind-dev

      - name: Smoke tests
        run: cargo test --all

      - name: Bolero property tests (fast mode)
        # Pipe output to triage — don't re-run bolero inside triage
        run: cargo bolero test --all 2>&1 | tee bolero-output.txt
        continue-on-error: true

      - name: Triage failures
        if: failure()
        env:
          DEEPINFRA_API_KEY: ${{ secrets.DEEPINFRA_API_KEY }}
        run: cat bolero-output.txt | kask qa triage

  # ── Deep path: merge to main only ─────────────────────────────────
  deep-qa:
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Install system deps
        run: sudo apt-get install -y binutils-dev libunwind-dev

      - name: Mutation testing (temp dir, not --in-place)
        run: cargo mutants --timeout 60 2>&1 | tee mutation-report.txt

      - name: Emit mutation score to CNS
        run: |
          SCORE=$(grep -oP 'Mutation score: \K[\d.]+' mutation-report.txt || echo "0")
          # CNS: cns.qa.metric { name: "mutation_score", value: SCORE }

      - name: Coverage-guided fuzzing
        run: cargo bolero test --all --engine libfuzzer --duration 300 2>&1 | tee bolero-deep-output.txt
        continue-on-error: true

      - name: Triage failures
        if: failure()
        env:
          DEEPINFRA_API_KEY: ${{ secrets.DEEPINFRA_API_KEY }}
        run: cat bolero-deep-output.txt | kask qa triage

      - name: Suggest fuzz targets from surviving mutants
        if: success()
        env:
          DEEPINFRA_API_KEY: ${{ secrets.DEEPINFRA_API_KEY }}
        run: |
          grep 'Uncaught mutants in:' mutation-report.txt | kask qa suggest-fuzz

      - name: Upload mutation report
        uses: actions/upload-artifact@v4
        with:
          name: mutation-report
          path: mutation-report.txt

  # ── Nightly: exhaustive fuzzing ───────────────────────────────────
  nightly-fuzz:
    if: github.event_name == 'schedule'
    runs-on: ubuntu-latest
    timeout-minutes: 120
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Install system deps
        run: sudo apt-get install -y binutils-dev libunwind-dev

      - name: Exhaustive fuzzing (2 hours)
        run: cargo bolero test --all --engine libfuzzer --duration 7200 2>&1 | tee bolero-nightly-output.txt
        continue-on-error: true

      - name: Triage failures
        if: failure()
        env:
          DEEPINFRA_API_KEY: ${{ secrets.DEEPINFRA_API_KEY }}
        run: cat bolero-nightly-output.txt | kask qa triage
```

---

## Cost Profile

| Operation | Model | Tokens | DeepInfra Cost | Frequency |
|-----------|-------|--------|---------------|-----------|
| Classify one bolero failure | Gemma 4 26B A4B | ~400 in, ~300 out | ~$0.00030 | Per failure |
| Feedback: rejected repair | Gemma 4 26B A4B | ~200 in, ~100 out | ~$0.00010 | Per rejection |
| Feedback: surviving mutant → fuzz suggestion | Gemma 4 26B A4B | ~200 in, ~200 out | ~$0.00016 | Per mutant |
| cargo-bolero (fast mode) | — | — | $0 | Every push/PR |
| cargo-bolero (libFuzzer) | — | — | $0 | Every merge |
| cargo-bolero (nightly) | — | — | $0 | Nightly |
| cargo-mutants (temp dir) | — | — | $0 | Every merge |

**Estimated monthly cost at hKask scale (~40 crates, typical bug rate):** <$0.10 in API fees.

---

## Success Criteria

| # | Criterion | How Verified |
|---|-----------|-------------|
| 0 | Current-condition audit complete | Numbers recorded: test count, unsafe count, pub fn count |
| 1 | ≥ 15 smoke tests across P0 crates | `grep -r '#\[test\]' crates/hkask-{types,cns,inference}/ --include='*.rs' | wc -l` ≥ 15 |
| 2 | ≥ 3 fuzz targets on substrate crates | `find crates/hkask-{types,cns,inference}/fuzz -name '*.rs' | wc -l` ≥ 3 |
| 3 | `cargo bolero test --all` completes in <60s (fast mode) | CI timing |
| 4 | Mutation score > 0% on at least one crate | `cargo mutants` output |
| 5 | `kask qa triage` classifies a bolero failure → CNS span emitted | CNS span log |
| 6 | `git apply --check` prevents broken diff application | Manual test with intentionally malformed diff |
| 7 | Duplicate repair branch blocked | Manual test with existing auto-heal branch |
| 8 | Unparseable classifier output opens human issue | Manual test with malformed JSON response |
| 9 | At least one feedback cycle completes (rejected PR → correction passage) | CNS span log |
| 10 | Zero new annotations in production code | `grep -r "#\[contract\]" crates/ --include="*.rs"` count unchanged |

---

## What This Does NOT Do

| Anti-pattern | Why Avoided |
|-------------|-------------|
| No `#[contract]` annotations | Removed — suffocated the code |
| No pre/post/invariant DSL | Same reason as above |
| No new model deployment | Uses existing Gemma 4 26B via `classify_batch` |
| No new binary to compile | `kask qa triage` is a CLI subcommand, reuses existing binary |
| No visual QA dashboard | P3 Prohibition #1 — CNS spans + CLI only |
| No Python QA scripts | AGENTS.md tooling policy — Rust only |
| No pass-through LLM wrappers | P5 deep-module discipline — each tool earns its existence |
| No auto-merge to main | P1 User Sovereignty — human always reviews the PR |
| No `--in-place` mutation in CI | Temp dir prevents working tree corruption on CI kill |
| No re-running bolero inside triage | Piped from CI — single execution, no redundant test time |
| No silent failure drops | Unparseable classifier output → human issue |
| No repair loops | Dedup guard checks for existing auto-heal branches/PRs |

---

## References

| Source | Relevance |
|--------|-----------|
| `hkask-services-classify` (`crates/hkask-services-classify/src/classify_impl.rs`) | Existing `classify_batch` API used by QA triage |
| `registry/classify/gemma-classifier.yaml` | Model config pattern for `qa-triage.yaml` and `qa-feedback.yaml` |
| `hkask-inference` (`crates/hkask-inference/src/inference_router.rs`) | Multi-provider inference router |
| `hkask-types/src/cns.rs` (`CnsSpan`) | CNS span registry — 5 QA spans added |
| `docs/architecture/core/TESTING_DISCIPLINE.md` | Testing philosophy — property-based testing foundation |
| gemma-classifier model card (`google/gemma-4-26B-A4B-it`) | 3.8B active, 26B total MoE, Apache 2.0, 77.1% LiveCodeBench v6 |
| bolero (`camshaft/bolero`, v0.13, MIT) | Unified fuzz + property + Kani front-end |
| cargo-mutants | Zero-config mutation testing |
| Toyota Kata (Rother, 2009) | 4-step scientific improvement pattern governing this plan |
