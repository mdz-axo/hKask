---
title: "hkask-guard — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "3d1a876f"
---

`hkask-guard` provides mandatory content safety guard at every LLM boundary, implementing P3.1 Social Generativity. Core scanners are always active — not configurable off. This is the floor, not the ceiling. The implementation is aligned with OWASP Top 10 for LLM Applications (LLM01, LLM02, LLM04, LLM06), NIST AI RMF 1.0, and ENISA's multilayer framework.

## Public Modules

| Module | Purpose |
|---|---|
| `pipeline` | `ContentGuard`, `GuardConfig`, `GuardResult`, `GuardViolation`, `GuardOutput` |

## Key Types

### `ContentGuard`

Mandatory content safety guard with two always-active pipelines:

- **Input pipeline**: Scans before model invocation for prompt injection, role override, and token limits.
- **Output pipeline**: Scans after model response for secret leakage; secrets are stripped before storage.

Constructed via `ContentGuard::mandatory(config)`. The `config` controls scanner parameters, not scanner presence. Built from `llm-guard` primitives: `BanSubstrings`, `Deobfuscate`, `Secrets`, `TokenLimit`, and `RoleOverride`.

### `GuardConfig`

Configuration controlling scanner parameters (not presence). All core scanners are always active.

| Field | Type | Purpose |
|---|---|---|
| `token_limit` | `usize` | Maximum input token budget before model invocation. Override via `HKASK_GUARD_TOKEN_LIMIT` env var. Default: 32,000. |

Implements `Default` which reads `HKASK_GUARD_TOKEN_LIMIT` from the environment, falling back to 32,000.

### `GuardResult`

Result of a content safety scan.

| Field | Type | Purpose |
|---|---|---|
| `passed` | `bool` | Whether content passed all mandatory checks |
| `violations` | `Vec<GuardViolation>` | Scanner name to description |
| `output` | `GuardOutput` | Output state — `Clean` or `Sanitized(String)` |

### `GuardOutput`

State of content after guard scanning. Enum variants:

| Variant | Purpose |
|---|---|
| `Clean` | Content passed all checks unchanged |
| `Sanitized(String)` | Content was modified — secrets were stripped |

Methods: `is_modified() -> bool`, `content(original: &str) -> &str` (returns sanitized content if modified, original otherwise).

### `GuardViolation`

A single guard violation. Fields: `scanner` (`String` — name of the scanner that flagged), `description` (`String` — human-readable violation description). Matched spans over `MAX_VIOLATION_SPAN_DISPLAY` (40 characters) are redacted to avoid logging sensitive content.

## OWASP LLM Risk Alignment

The guard's scanners are mapped to OWASP LLM risks:

| OWASP LLM Risk | Guard Scanner | Implementation |
|---|---|---|
| LLM01: Prompt Injection | `BanSubstrings` + `Deobfuscate` | Curated injection patterns with deobfuscation pre-pass (base64, leet, spacing, confusables) |
| LLM02: Insecure Output Handling | `Secrets` | Credential leak detection (API keys, JWTs, PEMs), stripped before shared memory storage |
| LLM04: Model Denial of Service | `TokenLimit` | 32K token budget gate before model invocation |
| LLM06: Sensitive Information Disclosure | `Secrets` (output) | Secrets in model output are redacted before entering any persistent store |

Future scanners available in `llm-guard` but not yet wired: `ScriptMix` (LLM03 — Unicode look-alike detection) and `InvisibleText` (LLM08 — zero-width/bidi character smuggling).

## Feature Flags

No feature flags are defined. This crate is a core dependency.
