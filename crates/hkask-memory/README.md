# hkask-memory

Semantic and episodic memory pipelines for hKask.

Implements the memory consolidation pipeline (L2 in the loop architecture):
episodic → consolidation → semantic.

## Forgetting Curve

Wozniak & Gorzelanczyk (1995), equation (3): **R(t) = exp(-t/S)**

Where S is memory life in days (configurable, default 180 = 6 months × 30).
After S days without recall, confidence decays to exp(-1) ≈ 36.8%.

At recall (when a memory is pulled into a prompt as context), the decay clock
resets — t goes back to 0, R = 1.0.

## Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `HKASK_MEMORY_LIFE_DAYS` | Memory life S in days | 180 |
| `HKASK_DB_PROVIDER` | Database provider (`sqlite` or `postgres`) | — |
| `HKASK_DB_PATH` | SQLite database path | — |
| `HKASK_DB_PASSPHRASE` | Database encryption passphrase | — |

`ServiceConfig.memory_life_days` also accepts the value programmatically.

## Template Pipeline

Two separate Jinja2 templates for hMem extraction from agent operations.

### Dual-Model Epistemic Integrity

Classification uses two peer models from different jurisdictions — neither is
primary. Both models receive the same few-shot prompt, and their extractions
are integrated with divergence detection. Classification with a single model
is not permitted — the system requires dual-model configuration and refuses
to operate without model B.

| Setting | Env Var | Default |
|---|---|---|
| Model A | `HKASK_CLASSIFIER_MODEL_A` | `KC/qwen/qwen3-235b-a22b-2507` (KiloCode, China) |
| Model B | `HKASK_CLASSIFIER_MODEL_B` | `DI/google/gemma-4-E4B-it` (DeepInfra, US) |

When models diverge (Jaccard < 0.6), the system emits a
`cns.classify.dual_fidelity` CNS alert. Drift detection (`cns.classify.drift`)
monitors extraction patterns over time for model behavior changes.

### Content Safety

All LLM boundaries are protected by `hkask-guard` — mandatory input/output
scanning aligned with OWASP LLM Top 10. Prompt injection, role override, and
secret leakage detection are always active at every classification call.

### Known Gap: Remember Templates

The `remember-episodic.j2` and `remember-semantic.j2` templates render through
the inference router directly, not through the dual-model classifier pipeline.
Agent memory formation currently uses single-model classification. This is a
**documented gap** — dual-model must be applied to template-based memory
extraction. The fix requires extending WordAct template model selection to
support dual-model routing.

### Memory Templates

Templates invoked via `memory_remember.yaml` FlowDef manifest with `dual_model: true`
on each step. The `ManifestExecutor` routes dual-model steps through two peer
inference ports, merges JSON outputs via case-insensitive set union, and stores
the integrated result.

| Template | Memory Type | Perspective | Extraction Focus |
|----------|-------------|-------------|-----------------|
| `remember-episodic.j2` | Episodic | First-person | Process, actions, observations |
| `remember-semantic.j2` | Semantic | Third-person | Facts, relationships, knowledge |

### Content Safety
The FlowDef selector (`operation-selector.j2`) classifies the request and
routes to the appropriate template.

## Consolidation

Episodic → Semantic is a one-way bridge. Consolidation:

1. Selects oldest, lowest-confidence episodic hMems
2. Decays confidence at point of use: `memory_decay(days_since, memory_life_days)`
3. Strips perspective, sets Public visibility
4. Bayesian combines with existing semantic hMems (log-odds pooling)
5. Expires episodic source (soft-delete via valid_to)

No token or authorization required — consolidation is always permitted.
