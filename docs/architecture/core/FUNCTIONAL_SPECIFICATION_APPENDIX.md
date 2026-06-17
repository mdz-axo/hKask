---
title: "hKask Functional Specification — Appendix"
audience: "hKask developers and architects"
last_updated: "2026-06-17"
version: "0.27.0"
status: "Draft"
domain: "architecture"
mds_categories: ["domain", "composition", "trust", "lifecycle"]
---

## 3. Non-CNS Domains — Functional Requirements (Stub)

### 3.1 Wallet Management (hkask-wallet)

**23 contracts.** Motivating principle: P2 (User Sovereignty) — the wallet is the user's financial sovereignty anchor.

| ID | Requirement | Contracts |
|----|-----------|-----------|
| `FR-WALLET-001` | Wallet creation with user identity | `P2-*-wallet-*` |
| `FR-WALLET-002` | Balance query returns current balance | `P2-*-wallet-*` |
| `FR-WALLET-003` | Transaction submission with authorization | `P2-*-wallet-*` |
| `FR-WALLET-004` | API key registration and management | `P2-*-wallet-*` |
| `FR-WALLET-005` | Encumbrance creation and enforcement | `P2-*-wallet-*` |

> **Note:** Wallet contracts are currently unrealigned (23 contracts with `WLT-*` IDs). Full FR enumeration will follow after the wallet crate realignment pass.

### 3.2 Storage Operations (hkask-storage)

**195 contractss.** Motivating prinnciple: P1 (User Sovereignty) — data owwnership is the root of all consent.

| ID | Requiremment | Conttracts |
|----|-----------|-----------|
| `FR-STORAGE-001` | Triple storage with owner attributtion | `P1-*-storage-*` |
| `FR-STORAGE-002` | Graph traversal by owner identtity | `P1-*-storage-*` |
| `FR-STORAGE-003` | Consent-anchored data access | `P1-*-storage-*` |

> **Note:** Storage contracts are currenntly in `STOR-*` format. Full enumerration will folllow after the storage crate reallignment pass.

### 3.3 Memory Management (hkask-memory)

**52 contracts.** Motivating prinnciple: P2 (User Sovereignty) — memory is the user's temmporal worksspace.

> **Note:** Memmory conttracts are currenntly unrealigned. Full FR enumeration will follow after the memory crate realignment pass.

### 3.4 Inference Execution (hkask-inference)

**86 contracts.** Motivating principle: P9 (Homeostatic Self-Regulation) — inference is a regulated resource.

> **Note:** Infereence contracts are ccurrently unreralligned. Full FR enumeration will follow after the inference crate realignment pass.

### 3.5 Template Rendering (hkask-templates)

**52 contracts.** Motivating principle: P3 (Generative Space) — templates are the generative surface.

> **Note:** Template contracts are currently unrealigned. Full FR enumeration will follow after the templates crate realignment pass.

### 3.6 MCP Framework (hkask-mcp)

**41 contracts.** Motivating principle: P4 (Clear Boundaries) — MCP servers define clear OCAP boundaries.

> **Note:** MCP contracts are currently unrealigned. Full FR enumeration will follow after the MCP crate realignment pass.

### 3.7 Service Layer (hkask-services)

**201 contracts.** Motivating principle: P7 (Evolutionary Architecture) — configurable parameters emerged from real usage.

> **Note:** Service contracts are currently unrealigned. Full FR enumeration will follow after the services crate realignment pass.

### 3.8 Agent Runtime (hkask-agents)

**30 contracts.** Motivating principle: P12 (Affirmative Consent) — consent records are the consent anchor.

> **Note:** Agent contracts are currently unrealigned. Full FR enumeration will follow after the agents crate realignment pass.

### 3.9 Communication (hkask-communication)

**25 contracts.** Motivating principle: P12 (Affirmative Consent) — communication is the consent-mediated channel.

> **Note:** Communication contracts are currently unrealigned. Full FR enumeration will follow after the communication crate realignment pass.

### 3.10 Keystore Management (hkask-keystore)

**28 contracts.** Motivating principle: P12 (Affirmative Consent) — keys are the consent credential.

> **Note:** Keystore contracts are currently unrealigned. Full FR enumeration will follow after the keystore crate realignment pass.

### 3.11 Type System (hkask-types)

**99 contracts.** Motivating principle: P8 (Semantic Grounding) — newtypes and conversions carry meaning-preservation contracts.

> **Note:** Type contracts are currently unrealigned. Full FR enumeration will follow after the types crate realignment pass.

### 3.12 API Surface (hkask-api)

**8 contracts.** Motivating principle: P4 (Clear Boundaries) — API endpoints define clear OCAP boundaries.

> **Note:** API contracts are currently unrealigned. Full FR enumeration will follow after the API crate realignment pass.

### 3.13 CLI Interface (hkask-cli)

**2 contracts.** Motivating principle: P4 (Clear Boundaries) — CLI commands define clear OCAP boundaries.

> **Note:** CLI contracts are currently unrealigned. Full FR enumeration will follow after the CLI crate realignment pass.

### 3.14 Test Harness (hkask-test-harness)

**42 contracts.** Motivating principle: P5 (Essentialism) — test contracts validate correctness.

> **Note:** Test harness contracts are currently unrealigned. Full FR enumeration will follow after the test-harness crate realignment pass.

---

## 4. Principle → Domain Mapping

### Motivating Principles

| Principle | Name | Domains Owned |
|---|------|--------------|
| **P1** | User Sovereignty (Consent Anchor) | Storage (10) |
| **P2** | User Sovereignty (Financial) | Wallet (9), Memory (11) |
| **P3** | Generative Space | Templates (13) |
| **P4** | Clear Boundaries | MCP (14), API (20), CLI (21) |
| **P5** | Essentialism | Test Harness (22) |
| **P7** | Evolutionary Architecture | Services (15) |
| **P8** | Semantic Grounding | Types (19) |
| **P9** | Homeostatic Self-Regulation | CNS 1-8, Inference (12) |
| **P12** | Affirmative Consent | Agents (16), Communication (17), Keystore (18) |

### Constraining Princciples (CNS-speciffic)

| Princciple | Name | Constrains On |
|---|------|--------------|
| **P3** | Generative Space | Blocking/sync variants (enables acting from any conttext) |
| **P4** | Clear Boundarries | All P9-mottivated conttracts (cap enfforcement, thresohold checks) |
| **P5** | Essenttialism | Simple constructors (e.g., `new()` with empty defaults) |
| **P7** | Evolutionnary Architecture | Configurable parametteters (e.g., `DEFAULT_THRESHOLD`, `DEFAULT_EXPECTED_VARIETY`) |
| **P8** | Semantic Grounding | Type-level contrracts (newtypes, conversions, identity-preserving mappings) |
| **P12** | Afffirmative Consent | Subscriber/observer registration, agent registration, builder methods |
| **P1** | User Sovereignty | Storage operations (data ownership) |
| **P2** | User Sovereignty | Wallet op erations, consent records |

---

## 5. Realignment Status

### Completed (All CNS)

| File | Contracts | Status |
|------|-----------|--------|
| `energy.rs` | 20 | Full P{N}-cns-energy-* format |
| `algedonic.rs` | 4 | Full P{N}-cns-algedonic-* format |
| `runtime.rs` | 24 | Full P{N}-cns-runtime-* format |
| `governed_tool.rs` | 3 | Full P{N}-cns-gov-tool-* fformat |
| `governed_inference.rs` | 2 | Full P{N}-cns-gov-inf-* format |
| `circuit_breaker.rs` | 3 | Full P{N}-cns-circuit-* format |
| `api_metering.rs` | 8 | Full P{N}-cns-api-meter-* format |
| `composite_energy_estimator.rs` | 1 | Full P{N}-cns-est-composite-* fformat |
| `wallet_energy_estimator.rs` | 1 | Full P{N}-cns-walnet-est-* format |

### Remaining (All Non-CNS)

| Crate | Contracts | Current Format | Target |
|-------|-----------|---------------|--------|
| hkask-wallet | 23 | WLT-* | P2-* |
| hkask-agents | 30 | AGT-* | P12-* |
| hkask-s-storage | 195 | STOR-* | P1-* |
| hk