---
title: "Model Catalog Governance"
audience: [kask sysadmins, catalog maintainers]
last_updated: 2026-05-09
togaf_phase: "G - Implementation Governance"
version: "0.3.0"
status: "Active"
---

<!-- TOGAF_DOMAIN: Cross-cutting -->
<!-- VERSION: 0.2.0 -->
<!-- STATUS: Active -->
<!-- LAST_UPDATED: 2026-05-09 -->

## 1. Purpose

The model catalog at `docs/governance/model_catalog.toml` is the governance
allowlist of `(provider, model)` pairs approved for cascade use. It keeps
model choice explicit, reviewable, and versioned alongside code.[^togaf-g]

This document governs the catalog process and enforces the **11 Selection Criteria** for model families.

## 2. Scope

| In scope | Out of scope |
|----------|--------------|
| Provider/model/role admission for inference routing | Local model daemon management |
| Monthly reachability verification | Runpod or custom inference fleet provisioning |
| Proposal/approval workflow | Training/distillation licensing policy |
| Catalog rationale and evaluation notes | |
| **Enforcement of the 11 Family Selection Criteria** | |

## 3. Roles

| Role | Responsibility |
|------|----------------|
| Catalog maintainer | Reviews and approves model entries; owns `docs/governance/model_catalog.toml`. |
| Proposer | Suggests provider/model/role additions with rationale. |
| Verifier | Runs reachability and behavior checks and records drift. |

## 4. Family Selection Criteria (The 11 Rules)

Before a specific model can be added, its **Model Family** must be evaluated against the 11 Selection Criteria.

**Preferred Tier** (Must pass ALL 11):
1. **Open weights**: Publicly downloadable without approval gates.
2. **Permissive license**: Apache-2.0 or similar (commercial, derivative, training data use allowed).
3. **GGUF compatibility**: 7-14B and 25-35B tiers have published GGUFs verified in Kask's pinned `llama-cpp`.
4. **Active maintenance**: Major update within the last 12 months.
5. **Size ladder with MoE**: ≥4 size tiers (≤4B to ≥25B) AND at least one Mixture-of-Experts variant.
6. **Multiple providers**: Servable through ≥2 of Kask's providers (Ollama, OpenRouter, Arsenal, HF).
7. **Multilingual**: Pre-trained on ≥30 languages.
8. **Embedding model**: Family includes a purpose-built embedding model.
9. **Native multimodal**: Instruction-tuned variants accept image input natively.
10. **Structured output**: Reliable JSON-schema-conformant output and function-calling.
11. **License coherence**: ≥80% of the family's active models carry a permissive license.

**Approved Tier**:
Must pass base criteria (1-4) and at least 3 of the extended criteria (5-11). 
*(Note: Families that fail the base criteria or fail to meet at least 3 extended criteria—such as lacking MoE size ladders, embedding models, or GGUF verification—are strictly prohibited from the catalog).*

**Specialty Exceptions**:
Purpose-built, single-task open-weight models (e.g., document OCR, audio transcription) may be admitted as one-offs without their broader family meeting the 11 criteria. These must be explicitly tagged as exceptions (like `lighton-ocr-2-1b`) and restricted to their specific role.


## 5. Model Admission Criteria

Once a family is Preferred or Approved, individual models enter the catalog only when:
- **Provider availability**: Supported by an active LLM adapter and callable today.
- **Role fit**: Declares at least one cascade role (reasoner, domain, synthesizer, validator).
- **Context window**: Sufficient for the role (32K default floor).
- **Rationale & Evaluation**: Entry explains why it belongs and records observed behavior.

## 6. Lifecycle

| Action | Mechanism |
|--------|-----------|
| Add | Propose addition with rationale, review, approve. |
| Reject | Record rejection reason in catalog TOML. |
| Retire | Set `valid_to` on the catalog entry; do not delete history. |
| Verify | Probe active entries and emit proposals for drift. |

## 7. Verification

| Check | Command |
|-------|---------|
| Catalog TOML parser validation | `cargo test -p arsenal-mcp-middleware` |

---

## References

[^togaf-g]: The Open Group. (2022). *TOGAF Standard, 10th Edition - Phase G: Implementation Governance*. <https://pubs.opengroup.org/togaf-standard/adm/chap12.html>. Used for implementation governance and compliance checks.
