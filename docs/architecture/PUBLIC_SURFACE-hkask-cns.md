---
title: "Public Surface Justification — hkask-cns"
audience: [architects, developers]
last_updated: 2026-06-15
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [composition]
---

# Public Surface Justification — hkask-cns

**Crate:** `hkask-cns`  
**Public items in lib.rs:** 25  
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Why This Surface Is Large

`hkask-cns` is the **Cybernetic Nervous System** — homeostatic self-regulation for the entire hKask ecosystem. Its surface is large because it implements multiple regulatory subsystems:

1. **CyberneticsLoop** — The main sense→compare→compute→act regulator with energy budget management, algedonic alerts, and loop-quality telemetry.
2. **Energy budget** — `EnergyBudget`, `EnergyCost`, `EnergyBudgetManager`, wallet-backed budgets, and gas accounting.
3. **GovernedTool** — OCAP-gated MCP tool invocation wrapper used by all MCP servers.
4. **Circuit breaker** — Inference circuit breaker with half-open recovery.
5. **Set points** — Configurable thresholds for variety, error rate, latency, and energy.

## Mitigations

- **Submodule organization:** Each regulatory concern (energy, governed_tool, circuit_breaker, dampener, set_points) is a separate module.
- **Trait-based loop:** `HkaskLoop` trait enables testability and future loop additions.

## Deletion Test

Delete `hkask-cns` and energy budget enforcement, tool governance, circuit breaking, and homeostatic regulation reappear scattered across every MCP server and inference path. The crate earns its existence.
