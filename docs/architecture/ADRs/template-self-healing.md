# Template Self-Healing Loop — Design

**Status:** Proposed | **Version:** 0.1.0 | **Date:** 2026-06-27

## Overview

When a KnowAct template invocation fails (render error, inference timeout,
malformed JSON, schema mismatch), the Curator can diagnose and repair the
template file on disk using the `template-healer.j2` KnowAct template,
acting as a coding agent with filesystem tools.

## The Loop

```
MetacognitionLoop::compute_with_templates()
  │
  ├─ execute_knowact("diagnose.j2", ctx)
  │   │
  │   ├─ OK, schema-valid → produce LoopActions
  │   │
  │   └─ ERR → TemplateHealLoop (below)
  │
  └─ Fallback: compute_with_thresholds() ← always available
```

```
TemplateHealLoop
  │
  ├─ 1. CAPTURE
  │     template_path: String     (e.g. "curator/metacognition-diagnose.j2")
  │     error_context: String     (full error chain)
  │     template_content: String  (read from disk)
  │     input_context: Value      (the HashMap passed to execute_knowact)
  │     failure_history: [Failure] (previous attempts)
  │
  ├─ 2. DIAGNOSE
  │     Invoke "curator/template-healer.j2" with capture context
  │     → LLM produces { diagnosis, fix_type, proposed_patch, confidence }
  │
  ├─ 3. APPROVAL GATE (P1: User Sovereignty, P2: Affirmative Consent)
  │     │
  │     ├─ confidence < 0.5 → escalate to user with diagnosis,
  │     │   template content, and proposed_patch. Await user decision.
  │     │
  │     ├─ confidence >= 0.5 → post proposed fix to escalation queue
  │     │   for user review. Apply only after explicit user approval.
  │     │
  │     └─ User rejects → log, increment failure_count, fallback to Rust
  │
  ├─ 4. APPLY (user-approved only)
  │     Write proposed_patch to registry/templates/curator/<template>.j2
  │     via filesystem MCP tool.
  │
  ├─ 5. RETRY
  │     Invoke execute_knowact() with same context against repaired template.
  │     │
  │     ├─ OK → clear failure_history, return LoopActions
  │     └─ ERR → increment attempt_count
  │         │
  │         ├─ attempt_count < 3 → loop back to CAPTURE (enriched history)
  │         └─ attempt_count >= 3 → escalate to user, fallback to Rust
  │
  └─ Max 3 repair attempts per template per cycle.
```

## Required Infrastructure

| Component | Status | What's Needed |
|-----------|--------|--------------|
| `curator/template-healer.j2` | ✅ Created | KnowAct template for diagnosis + repair |
| Filesystem MCP tool (read) | ✅ Exists | `ManifestExecutor` can read template files from `template_base_path` |
| Filesystem MCP tool (write) | ❌ Needed | MCP tool to write modified `.j2` files to `registry/templates/` |
| User approval mechanism | ⚠️ Partial | Escalation queue exists; needs "proposed_patch" review UI |
| Failure history tracking | ❌ Needed | Per-template failure counter + last N error contexts |
| Confidence gate | ⚠️ Partial | `CuratorDirective` types exist; need `ProposeTemplateFix` variant |

## Integration Point

The heal loop integrates at `MetacognitionLoop::compute_with_templates()`
line ~610, where `execute_knowact()` errors are currently caught with:

```rust
Err(e) => {
    tracing::warn!(..., "Template failed; fallback to thresholds");
    return self.compute_with_thresholds(deviations);
}
```

The heal loop replaces this with a capture → diagnose → approve → apply → retry
sequence before falling back.

## Design Constraints

1. **User sovereignty (P1):** Templates are generative artifacts owned by the
   user. No autonomous modification without explicit approval.
2. **Fail-closed (P2):** If the heal loop fails, fall back to Rust thresholds.
   Never leave the system without a regulatory path.
3. **Bounded retry:** Max 3 repair attempts per template per cycle.
4. **Minimal diffs:** Patches must be specific, not template rewrites.
5. **Audit trail:** Every heal attempt logged as a CNS `cns.curation.template_heal` span.
