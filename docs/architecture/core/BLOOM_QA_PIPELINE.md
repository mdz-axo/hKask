# Capabilities Researcher — Bloom QA Pipeline

**Version:** 4.0 | **Pipeline:** `corpus/pipeline-capabilities-researcher.yaml`

## Persona

**Business and Economics Researcher** — analyzes the gap between what organizations, markets, and systems are capable of and what they actually achieve. Draws on economic theory, systems thinking, computing principles, scientific method, and institutional analysis. Core question: *What is the economic significance of unrealized potential?*

## Analytical Framework

```
Capabilities (what a system CAN do)
    ├── Economic: transaction costs, agency, information asymmetry
    ├── Systems: feedback loops, emergence, path dependence
    ├── Computing: information theory, algorithmic efficiency
    ├── Scientific: hypothesis testing, empirical evidence
    └── Institutional: culture, governance, historical context
    │
    ▼
Performance (what a system ACTUALLY achieves)
    │
    ▼
GAP ← Economic significance? Why does it exist? How to close it?
```

## Bloom's Taxonomy (Capability-Performance Frame)

| Level | Application |
|-------|------------|
| **Factual** | Identify capabilities, resources, performance metrics, gap measurements |
| **Conceptual** | Explain mechanisms linking capabilities to outcomes. What models fit? |
| **Analyze** | Compare capability-performance relationships across contexts. Find patterns. |
| **Evaluate** | Assess evidence for gap explanations. Critique frameworks. Judge significance. |
| **Create** | Design interventions. Synthesize multi-domain strategies. Formulate hypotheses. |

## Pipeline (8 Phases)

| Phase | Input | Output |
|-------|-------|--------|
| 0 | 105 source files (PDF/HTML) | Extracted text |
| 1 | Extracted text | Chunks (~500 tokens) |
| 2 | Chunks | Embeddings + salience tags |
| 3 | Tagged chunks | Bloom taxonomy prompts (3× per chunk) |
| 4 | Prompts | Generated QAs (DeepSeek V4 Pro) |
| 5 | Raw QAs | Balanced train/val/test (1,000/level) |
| 6 | Training set | h_mems + embedding vectors |
| 7 | Embeddings | John Brooks persona centroids |
| 8 | Chat format QAs | LoRA adapter (Qwen3.6-27B, RunPod/Unsloth) |

## Artifacts

| File | Purpose |
|------|---------|
| `corpus/qa_pairs/prompts_bloom.jsonl` | Bloom taxonomy prompts |
| `corpus/qa_pairs/train_chat.jsonl` | Training QAs (chat format) |
| `corpus/qa_pairs/val_chat.jsonl` | Validation QAs |
| `corpus/qa_pairs/test_chat.jsonl` | Test QAs |
| `corpus/memory/corpus_memory.db` | Semantic memory + embeddings |
| `corpus/replica/john-brooks.yaml` | Persona build config |
