---
name: sequential-thinking
visibility: public
description: "DEPRECATED — merged into sequential-inquiry. Use `sequential-inquiry` for all structured reasoning. The inquiry engine provides the same chain-of-thought protocol plus automatic deep-dive delegation to hypothesis-framer, mcda, and diagnose. No pre-selection needed."
---

# Sequential Thinking — DEPRECATED

This skill has been merged into **sequential-inquiry**.

The sequential-inquiry engine provides the full sequential thinking protocol (branching, revision, hypothesis testing) AND automatic delegation to deep-dive sub-skills. The engine decides at runtime whether delegation is needed — no pre-selection required.

**Use `kask run sequential-inquiry` for all structured reasoning tasks.**

## Migration

| Old | New |
|-----|-----|
| `kask run sequential-thinking` | `kask run sequential-inquiry` |
| 3-step PDCA, 8-criterion convergence | 6-step PDCA, 10-criterion convergence (delegation criteria cycle-aware) |
| Pure CoT reasoning | CoT reasoning + optional FINER+PICO, MCDA, or diagnose delegation |
| gas.cap: 100,000 | gas.cap: 120,000 |
| rjoule.cap: 1 | rjoule.cap: 2 |
