---
title: "Model Catalog Governance"
audience: [kask sysadmins, catalog maintainers]
last_updated: 2026-05-09
version: "0.3.0"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [curation]
---


## 1. Purpose

The model catalog at `docs/governance/model_catalog.toml` is the governance
allowlist of `(provider, model)` pairs approved for cascade use. It keeps
model choice explicit, reviewable, and versioned alongside code.[^togaf-g]

Model catalog governance follows established practices for LLM deployment management.[^chang2024]

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
3. **GGUF compatibility**: 7-14B and 25-35B tiers have published GGUFs verified in Kask's pinned `llama-cpp`.[^gguf-spec]
4. **Active maintenance**: Major update within the last 12 months.
5. **Size ladder with MoE**: ≥4 size tiers (≤4B to ≥25B) AND at least one Mixture-of-Experts variant.[^fedus2024]
6. **Multiple providers**: Servable through ≥2 of Kask's providers (Ollama, OpenRouter, Arsenal, HF).
7. **Multilingual**: Pre-trained on ≥30 languages.
8. **Embedding model**: Family includes a purpose-built embedding model.
9. **Native multimodal**: Instruction-tuned variants accept image input natively.
10. **Structured output**: Reliable JSON-schema-conformant output and function-calling.[^function-calling]
11. **License coherence**: ≥80% of the family's active models carry a permissive license.

**Approved Tier**:
Must pass base criteria (1-4) and at least 3 of the extended criteria (5-11). 
*(Note: Families that fail the base criteria or fail to meet at least 3 extended criteria—such as lacking MoE size ladders, embedding models, or GGUF verification—are strictly prohibited from the catalog).*

**Specialty Exceptions**:
Purpose-built, single-task open-weight models (e.g., document OCR, audio transcription) may be admitted as one-offs without their broader family meeting the 11 criteria. These must be explicitly tagged as exceptions (like `lighton-ocr-2-1b`) and restricted to their specific role. Quantization methods such as GPTQ[^frantar2022] and GGUF[^gguf-spec] enable efficient local deployment of specialty models.


## 5. Model Admission Criteria

Model evaluation follows standardized benchmarking practices for LLM assessment.[^hendrycks2021]

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

[^togaf-g]: The Open Group. (2022). *TOGAF Standard, 10th Edition - Phase G: Implementation Governance*. https://pubs.opengroup.org/togaf-standard/adm/chap12.html. Used for implementation governance and compliance checks.
[^gguf-spec]: Gerganov, G. (2023). *GGUF: GGML Universal File Format*. https://github.com/ggerganov/ggml/blob/master/docs/gguf.md
[^chang2024]: Chang, Y., Wang, X., Wang, J., Wu, Y., Yang, L., Zhu, K., Chen, H., Yi, X., Wang, C., Wang, Y., Ye, W., Zhang, Y., Han, Y., & Zhou, H. (2024). A survey on evaluation of large language models. *ACM Transactions on Intelligent Systems and Technology*, 15(3), 1–45. https://doi.org/10.1145/3641289
[^frantar2022]: Frantar, E., Ashkboos, S., Hoefler, T., & Alistarh, D. (2022). GPTQ: Accurate post-training quantization for generative pre-trained transformers. *arXiv preprint arXiv:2210.17323*. https://arxiv.org/abs/2210.17323
[^hendrycks2021]: Hendrycks, D., Burns, C., Basart, S., Zou, A., Mazeika, M., Song, D., & Steinhardt, J. (2021). Measuring massive multitask language understanding. *Proceedings of the International Conference on Learning Representations (ICLR 2021)*. https://arxiv.org/abs/2009.03300
[^fedus2024]: Fedus, W., Snell, J., Costa, T., & Zoph, B. (2024). Mixture-of-experts meets instruction tuning: Creating specialized experts from a generalist model. *arXiv preprint arXiv:2405.14764*. https://arxiv.org/abs/2405.14764
[^function-calling]: OpenAI. (2024). *Function calling and structured outputs*. https://platform.openai.com/docs/guides/function-calling
