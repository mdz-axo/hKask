---
title: "Replica, Corpus, and Training Readiness"
audience: [operators, developers, architects]
last_updated: 2026-07-10
version: "0.31.1"
status: "Active"
domain: "Training"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# Replica, Corpus, and Training Readiness

**Verdict:** the source corpus and early QA artifacts exist, but the John Brooks replica and the Qwen3.6-27B RunPod/Unsloth workflow are **not ready for unattended or production use through the MCP servers**. This is a code-and-artifact status report, not a forecast.

See the readiness flowchart (inlined in this document) for the verified pipeline boundary.

## Observed artifacts

| Stage | Verified state | Evidence |
|---|---|---|
| Sources | 105 files: 54 PDF, 49 HTML, 2 Markdown | `/home/mdz-axolotl/Clones/Library/Researcher` inventory, 2026-07-10 |
| Extraction | 70 extracted files under `corpus/extracted/researcher` | filesystem inventory |
| Chunking and salience | 4,734 records in each chunks JSONL file | `corpus/chunks/{chunks,tagged_chunks}.jsonl` |
| Prompt construction | 957 Bloom prompts | `corpus/qa_pairs/prompts_bloom.jsonl` |
| QA generation | 220 generated records | `corpus/qa_pairs/gen_bloom_all.jsonl` |
| Balanced datasets | absent: no `train_chat.jsonl`, `val_chat.jsonl`, or `test_chat.jsonl` | `corpus/qa_pairs/` inventory |
| John Brooks replica | configuration present; built centroid and quality-gate evidence absent | `corpus/replica/john-brooks.yaml`; `corpus/pipeline-capabilities-researcher.yaml` |
| RunPod adapter | no verified job, output artifact, or adapter registration evidence | training server review, 2026-07-10 |

The pipeline manifest targets 5,000 balanced QAs, so the current 220 generated records are **4.4% of the target**. That calculation describes artifact count only; it does not establish QA quality.

## Blocking implementation boundaries

| Boundary | Current implementation | Consequence |
|---|---|---|
| Docproc embedding/query | Indexing uses configured model selection; query can use a different hard-coded fallback and tolerates dimension mismatch | Retrieval can return zero-similarity rankings without an error |
| Docproc QA | Request-level model selection, typed QA validation, and response provenance are implemented; FlowDef execution and batch quality scoring remain incomplete | Generated records are structurally admissible, but are not yet a balanced, source-scored training dataset |
| Durable corpus handoff | `corpus-ingest embed` now persists chunk text and model/dimension/source provenance beside each vector; Docproc's own index remains process-local | Durable hydration is available through corpus ingestion, but no shared durable Docproc query path exists |
| FlowDef dispatch | Replica FlowDef dispatches its four owned corpus operations through typed request deserialization and explicit CLI arguments | `docproc_*`, generation, replica, script, and training steps remain external and the full manifest still cannot execute end-to-end |
| QA admission | Ingestion normalizes existing envelope records and requires source/chunk provenance plus exact quotations found in the canonical source chunk before it writes data | John Brooks persona scoring remains absent |
| Replica authority | High-authority MCP tools write files, mutate databases, and run subprocesses without per-tool authorization | The MCP surface is unsafe to expose beyond a trusted local operator |
| Replica quality | Discovery describes work instead of executing it; comparison/validation is bypassable or reporting-only | A configured John Brooks persona is not a verified replica |
| RunPod execution | Empty dataset handoff now fails before pod creation, and stopped/terminated pods are treated as failed without an artifact manifest | Per-job dataset URLs, durable provider IDs, and artifact-backed completion remain incomplete |
| Unsloth data contract | The rendered harness expects `text`; normalized records contain `messages` | The rendered training script cannot consume the server-produced dataset as written |

## Smallest safe remediation sequence

Each item is intentionally one independently testable contract. Do not begin a later item until the preceding item has passed its focused test.

1. **Unify docproc embedding identity.** Persist the model identifier and vector dimension with each indexed passage; reject query vectors with a mismatch.
2. **Completed: repair Docproc QA admission.** `HKASK_TEMPLATE_ROOT` resolves `registry/templates/docproc`; QA requests can select a provider-prefixed model and only admit schema-valid, Bloom-valid, cited cross-reference output with provenance.
3. **Verify the existing durable corpus write path.** Add a focused public-seam test proving `corpus_embed` persists a corpus chunk and that `replica_pipeline_run` reports unsupported external steps without checkpointing them as complete.
3. **Completed: durable corpus chunk hydration.** `corpus-ingest embed` stores each vector plus `text` and `corpus_provenance` h_mems under the same entity reference.
4. **Completed: typed corpus FlowDef dispatch.** The replica executor deserializes the four supported corpus requests and builds their explicit CLI contracts; no arbitrary JSON-to-argv conversion remains.
5. **Add persona scoring.** Extract reusable replica-comparison logic into a service and enforce the configured John Brooks threshold before admission.
6. **Constrain replica authority.** Require a capability/consent check for database mutation, cache writes, and subprocess execution; reject paths outside an approved root.
7. **Make replica discovery truthful.** Either execute the manifest and its consent gate or rename the tool to describe-only planning.
8. **Persist the RunPod lifecycle.** Store provider pod ID and per-job dataset/output artifact identities before reporting submission success.
9. **Make RunPod artifact-backed.** Replace the global dataset URL with private per-job handoff, validate a completion manifest, and only then permit adapter registration.
10. **Make Unsloth consume normalized data.** Format `messages` through the selected model chat template, explicitly set the Qwen thinking policy, and test the rendered script against a fixture.
11. **Only then submit a small pilot.** Use a private, disposable dataset and require an adapter artifact plus evaluation result before marking a job complete.

## Adversarial review notes

- **Essentialist:** retain the OCR executor boundary because it hides materially different backends. In contrast, the unused GOLEM bridge and metadata-only training lifecycle tools do not currently earn their surface area.
- **Grill-me:** “Which persisted record proves this exact corpus revision, model, provider pod, dataset, and adapter produced the observed quality result?” The current MCP workflow cannot answer this end-to-end.
- **Cybernetic:** `PipelineRunner` checkpoints per-step success or failure, but its replica executor stops at the first unsupported external tool. The loop therefore has local state feedback but no closed dispatch path for Docproc or training steps. Reporting a terminated pod as completed is positive-feedback risk, not regulation.
- **Semantic classification:** the artifact counts above are **descriptive/declarative observations**. The remediation order is a **prescriptive guideline** based on the source-level review. Readiness beyond this checkout remains a **hypothesis** until a complete run records its evidence.

## Validation scope

- `cargo test -p hkask-mcp-replica` passed in a focused run: 15 passed, 1 ignored.
- `cargo test -p hkask-mcp-docproc` passed in a focused run: 69 unit and 3 dependency-skipping integration tests.
- `cargo test -p hkask-mcp-training` passed in a focused run: 44 tests.
- A combined three-package test invocation was not conclusive because the workspace `target/` directory disappeared during compilation (`os error 2`), not because of a reported test failure.
- `docs/ci/verify-docs.sh` exposes pre-existing broad documentation failures and timed out while compiling doctests; this report does not claim the documentation corpus is globally clean.
