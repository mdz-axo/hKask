---
name: diagnose
description: Disciplined diagnosis loop for hard bugs and performance regressions. Reproduce → minimise → hypothesise → instrument → fix → regression-test. Use when user says "diagnose this" / "debug this", reports a bug, says something is broken/throwing/failing, or describes a performance regression.
---

# Diagnose

A discipline for hard bugs. Skip phases only when explicitly justified.

## Phase 1 — Build a Feedback Loop

**This is the skill.** Everything else is mechanical. If you have a fast, deterministic, agent-runnable pass/fail signal for the bug, you will find the cause. If you don't have one, no amount of staring at code will save you.

Spend disproportionate effort here. **Be aggressive. Be creative. Refuse to give up.**

### Ways to construct one — try them in roughly this order

1. **Failing test** at whatever seam reaches the bug — unit, integration, e2e.
2. **`cargo test` with a specific test name** — the simplest loop for Rust code.
3. **CLI invocation** with a fixture input, diffing stdout against a known-good snapshot.
4. **HTTP script** against the API server using `curl` or `reqwest`.
5. **Replay a captured input** — save a real payload to disk; replay it through the code path in isolation.
6. **Throwaway harness** — spin up a minimal subset of the system that exercises the bug code path with a single function call.
7. **Property / fuzz loop** — if the bug is "sometimes wrong output", run 1000 random inputs (`proptest`, `cargo fuzz`).
8. **Bisection harness** — if the bug appeared between two known commits, use `git bisect run`.
9. **Differential loop** — run the same input through old vs new version and diff outputs.

### Iterate on the loop itself

- Can I make it faster? (Skip unrelated init, narrow scope, use `--lib`.)
- Can I make the signal sharper? (Assert on the specific symptom, not "didn't crash".)
- Can I make it more deterministic? (Pin time, seed RNG, isolate filesystem.)

A 30-second flaky loop is barely better than no loop. A 2-second deterministic loop is a debugging superpower.

### Non-deterministic bugs

The goal is not a clean repro but a **higher reproduction rate**. Loop 100×, parallelise, add stress. A 50%-flake bug is debuggable; 1% is not.

### When you genuinely cannot build a loop

Stop and say so. Ask the user for: (a) access to whatever environment reproduces it, (b) a captured artifact (log dump, core dump), or (c) permission to add temporary instrumentation. Do **not** proceed to hypothesise without a loop.

## Phase 2 — Reproduce

Run the loop. Watch the bug appear. Confirm:

- [ ] The loop produces the failure mode the **user** described
- [ ] The failure is reproducible (or at a high enough rate)
- [ ] You have captured the exact symptom for later verification

## Phase 3 — Hypothesise

Generate **3–5 ranked hypotheses** before testing any. Single-hypothesis generation anchors on the first plausible idea.

Each must be **falsifiable**: "If X is the cause, then changing Y will make the bug disappear."

If you cannot state a prediction, the hypothesis is a vibe — discard or sharpen it.

**Show the ranked list to the user before testing.** They often have domain knowledge that re-ranks instantly.

## Phase 4 — Instrument

Each probe must map to a specific hypothesis from Phase 3. **Change one variable at a time.**

Tool preference:
1. **Debugger** — `rust-lldb` / `rust-gdb` or IDE breakpoint. One breakpoint beats ten logs.
2. **Targeted logs** with a unique prefix like `[DIAG-a4f2]`. Cleanup becomes a single grep.
3. **`RUST_LOG`** per-module tracing. Never "log everything and grep."
4. **Tag every debug log** with a unique prefix. Untagged logs survive; tagged logs die.

For performance: establish a baseline measurement (`cargo bench`, `criterion`, `flamegraph`), then bisect. Measure first, fix second.

## Phase 5 — Fix + Regression Test

Write the regression test **before the fix** — but only if there is a **correct seam** for it.

A correct seam exercises the **real bug pattern** as it occurs at the call site. If no correct seam exists, that itself is the finding — flag it for architecture review.

If a correct seam exists:
1. Turn the minimised repro into a failing test.
2. Watch it fail.
3. Apply the fix.
4. Watch it pass.
5. Re-run the Phase 1 feedback loop.

## Phase 6 — Cleanup + Post-mortem

- [ ] Original repro no longer reproduces
- [ ] Regression test passes (or absence of seam is documented)
- [ ] All `[DIAG-...]` instrumentation removed
- [ ] Throwaway prototypes deleted
- [ ] The correct hypothesis stated in commit/PR message
- [ ] `cargo clippy -p <crate> -- -D warnings` passes
- [ ] `cargo test -p <crate>` passes

**Then ask: what would have prevented this bug?** If the answer involves architectural change (no good test seam, tangled callers, hidden coupling), note it for architecture review — after the fix, not before.