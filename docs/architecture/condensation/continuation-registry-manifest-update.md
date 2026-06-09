# Condensation Continuation — Registry Manifest Inference Reference Update

**Status:** Pending. ~90 registry manifest YAML files still reference `hkask-mcp-inference` for LLM invocation steps. Inference is now internal (via `InferencePort` trait). These manifests need to be updated to reflect the new dispatch path.

---

## Background

The MCP server consolidation removed `hkask-mcp-inference` (inference is now internal cognition via `InferencePort`). Registry manifests are declarative pipeline definitions that reference MCP tools. ~90 manifests include steps like:

```yaml
- ordinal: 3
  action: execute
  target: generate_response
  mcp: hkask-mcp-inference
  tool: inference_generate
```

These references are stale — the server no longer exists. They were tagged with `# inference is now internal via InferencePort` comments during the consolidation cleanup, but the `mcp:` field still points to a non-existent server.

## Target State

All `mcp: hkask-mcp-inference` references in `registry/manifests/` should be replaced with the appropriate mechanism for the new inference dispatch path.

### Options

| Option | Description | Trade-off |
|--------|-------------|-----------|
| A | Replace `mcp: hkask-mcp-inference` with `mcp: hkask-mcp-condenser` (condenser wraps inference calls internally) | Condenser becomes inference gateway; changes condenser's responsibility |
| B | Add an `inference:` field to the manifest schema distinct from `mcp:` | Requires schema change; cleanest separation |
| C | Replace with `internal: inference` and handle inference dispatch outside the MCP tool system | Inference is no longer a tool; manifests describe conceptual steps |
| D | Remove inference steps from manifests — inference is implicit in any LLM-mediated pipeline step | Minimal change; inference becomes ambient infrastructure |

## Approach

### Phase 1 — Audit

Count and categorize all inference references:

```bash
grep -rn "hkask-mcp-inference" registry/manifests/ --include="*.yaml" | wc -l
# Categorize by pattern:
grep -rn "mcp: hkask-mcp-inference" registry/manifests/ --include="*.yaml" | wc -l
grep -rn "hkask-mcp-inference" registry/manifests/ --include="*.yaml" | grep -v "mcp:" | wc -l
```

### Phase 2 — Choose replacement strategy

Based on audit results, choose the appropriate option (A-D above) or a hybrid. The decision should consider:
- How inference is dispatched in the new architecture (via `InferencePort`, not MCP)
- Whether manifests should model inference as a tool step or as ambient infrastructure
- Backward compatibility for existing pipeline executions

### Phase 3 — Implement replacement

1. Update all manifest files with the chosen replacement
2. Update manifest schema if needed
3. Update any manifest validation/parsing code that checks for `hkask-mcp-inference`
4. Remove the `# inference is now internal via InferencePort` comment tags

### Phase 4 — Verify

```bash
# Zero remaining hkask-mcp-inference references
grep -rn "hkask-mcp-inference" registry/ --include="*.yaml" | grep -v "continuation"
# All manifests parse correctly (if schema validation exists)
# Pipeline execution works with new dispatch path
```

---

## Key Files

| File | Purpose |
|------|--------|
| `registry/manifests/*.yaml` | ~90 pipeline manifests with stale inference references |
| `registry/registries/**/*.yaml` | Registry index manifests (may also reference inference) |
| `crates/hkask-templates/src/` | Manifest schema definitions and parsing |
| `crates/hkask-services/src/inference.rs` | `InferenceService` — the new direct inference path |
| `crates/hkask-types/src/ports/mod.rs` | `InferencePort` trait — the inference abstraction |

## Affected Manifests (representative sample)

```
registry/manifests/adversarial-red-team.yaml
registry/manifests/chain-of-density.yaml
registry/manifests/coaching-kata.yaml
registry/manifests/coding-guidelines.yaml
registry/manifests/composition.yaml
registry/manifests/curator-metacognition.yaml
registry/manifests/decision-journal-revisit.yaml
registry/manifests/decision-journal.yaml
registry/manifests/ellipsis-analysis.yaml
registry/manifests/falstaffian-perspective.yaml
registry/manifests/grill-me.yaml
registry/manifests/handoff.yaml
registry/manifests/hemingway-style-synthesizer.yaml
registry/manifests/improvement-kata.yaml
registry/manifests/inference-dispatch.yaml
registry/manifests/kata-iteration.yaml
registry/manifests/kata-pattern.yaml
registry/manifests/mcda.yaml
registry/manifests/mcp_condense_session.yaml
registry/manifests/mcp_inference_call.yaml
registry/manifests/memory_recall.yaml
registry/manifests/metacognition.yaml
registry/manifests/prompt-injection-diagnostic.yaml
registry/manifests/rag-pipeline.yaml
registry/manifests/reasoning-cycle.yaml
registry/manifests/root-cause-analysis.yaml
registry/manifests/scenario-planning.yaml
registry/manifests/self-critique-revision-iteration.yaml
registry/manifests/self-critique-revision.yaml
registry/manifests/starter-kata.yaml
registry/manifests/structured-extraction.yaml
registry/manifests/superforecasting-pipeline.yaml
registry/manifests/tdd.yaml
registry/manifests/tool_dispatch.yaml
registry/registries/pragmatic-composition/process_manifest.yaml
```

---

*This continuation prompt captures all context needed for the registry manifest inference reference update.*
