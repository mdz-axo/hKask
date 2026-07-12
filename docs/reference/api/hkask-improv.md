---
title: "hkask-improv — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "3d1a876f"
---

# hkask-improv — API Reference

**Purpose:** Constructive interaction grammar for hKask agents. Five improv modes provide structured collaborative escalation protocols for dual-presence chat and kata coaching loops.

## Public Modules

| Module | Purpose |
|--------|---------|
| `modes` | `ImprovMode` enum — the five improv modes |
| `protocol` | Core protocol types: `ImprovResponse`, `Contribution`, `ConversationContext` |
| `plussing` | Plussing mode: constructive, additive collaboration |
| `riffing` | Riffing mode: exploratory variation on a theme |
| `freestyling` | Freestyling mode: unconstrained creative generation |
| `cascade` | `ImprovCascade` — mode escalation logic |

## Key Types

| Type | Description |
|------|-------------|
| `ImprovSkill` | The top-level improv skill — orchestrates mode selection and response generation |
| `ImprovMode` | Enum: `Plussing`, `YesAnd`, `YesBut`, `Freestyling`, `Riffing` |
| `ImprovResponse` | A structured response from an improv mode — contains the contribution and metadata |
| `Contribution` | A single contributed idea or suggestion |
| `ConversationContext` | The current state of the improv conversation (history, active mode, participants) |
| `ImprovCascade` | Escalation path: modes can cascade from one to another as the conversation evolves |
| `FreestyleSession` | State for an active freestyling session |

## Public API (7 items)

Per deep-module discipline, the improv crate exposes exactly 7 public items:
1. `ImprovSkill`
2. `ImprovMode`
3. `ImprovCascade`
4. `ImprovResponse`
5. `Contribution`
6. `ConversationContext`
7. `FreestyleSession`

## Mode Descriptions

| Mode | Behavior |
|------|----------|
| `Plussing` | "Yes, and…" — accepts and extends the previous contribution |
| `YesAnd` | Explicit acceptance with additive extension |
| `YesBut` | Conditional acceptance with constructive constraint |
| `Freestyling` | Open-ended creative generation without constraints |
| `Riffing` | Thematic variation — explores variations on a central theme |
