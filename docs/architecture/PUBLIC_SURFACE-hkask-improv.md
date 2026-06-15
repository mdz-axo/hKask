---
title: "Public Surface Justification — hkask-improv"
audience: [architects, developers]
last_updated: 2026-06-15
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [composition]
---

# Public Surface Justification — hkask-improv

**Crate:** `hkask-improv`  
**Public items in lib.rs:** 22  
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Why This Surface Is Large

`hkask-improv` is the **composable interaction grammar** — Plussing, Yes And, Yes But, Freestyling, Riffing, and Cascade modes for agent-to-agent and human-to-agent communication. Its surface is large because it implements multiple interaction protocols:

1. **Five improv modes** — Each mode (Plussing, YesAnd, YesBut, Freestyling, Riffing) has its own state machine, prompt template, and transition rules.
2. **Cascade** — Multi-agent composition mode with depth tracking and coherence scoring.
3. **REPL integration** — `/improv` slash command handlers for interactive mode switching.
4. **CNS spans** — Each mode emits `cns.improv.*` spans for observability.

## Mitigations

- **Mode isolation:** Each improv mode is a separate module with its own state and rules.
- **Shared traits:** `ImprovMode` trait enables uniform dispatch across modes.

## Deletion Test

Delete `hkask-improv` and the interaction grammar, mode state machines, and cascade composition reappear scattered across chat handlers and agent communication paths. The crate earns its existence.
