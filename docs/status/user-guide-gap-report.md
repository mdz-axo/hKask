---
title: "User Guide Gap Audit Report"
audience: [curators, documentation maintainers]
last_updated: 2026-06-17
version: "0.27.0"
status: "Active"
domain: "Documentation"
mds_categories: [curation]
---

# User Guide Gap Audit Report

Grill-me posture: skeptical new-user perspective. Every guide interrogated for onboarding clarity, success signals, failure modes, examples, cross-references, and missing sections.

---

## 1. Per-Guide Audits

### 1.1 `kanban-user-guide.md`

| Dimension | Assessment |
|-----------|-----------|
| **First command clear?** | Yes — `/kanban board create "My Project" --template software-project` is clear. |
| **What success looks like?** | Implicit — task moves through statuses. No explicit "you know it worked when..." statement. |
| **Failure modes?** | De-jamming section covers stuck tasks but not: board creation failure, invalid template, populate with malformed JSON, permission denied on assignment. |
| **Examples with expected output?** | Partial — commands shown but no expected terminal output. A new user doesn't know what a successful `view` looks like. |
| **Cross-references?** | Katas on Tasks § links to kata conceptually but not to `kata-user-guide.md`. Architecture § mentions P12/P1/P4/CNS but no links. No link to REPLICANT-ONBOARDING-WALKTHROUGH for setup. |
| **Missing sections** | **Critical:** No error recovery section. No "getting started flow" connecting kanban→kata→CNS. No troubleshooting. No "What if the LLM returns invalid JSON from decompose?" workflow. No explanation of the decompose→populate two-step flow rationale. |

**Score: 6/10** — Good core mechanics, weak on user safety and cross-guide integration.

### 1.2 `kata-user-guide.md` (`docs/guides/`)

| Dimension | Assessment |
|-----------|-----------|
| **First command clear?** | Yes — `kask kata start starter-kata --bot Alice`. Clear adoption path timeline. |
| **What success looks like?** | Yes — graduation criteria (automaticity > 0.5), "what to expect" per section. |
| **Failure modes?** | Anti-patterns § covers process mistakes. No technical failure modes (template render failure, CNS span missing, consent denied, model unavailable). |
| **Examples with expected output?** | Partial — commands given but no terminal output examples for starter/improvement/coaching runs. |
| **Cross-references?** | Excellent internally (IK↔CK composition rules table). No link to kanban-user-guide for the kanban→kata integration (`/kanban coach`, `/kanban improve`, `/kanban practice`). No link to CNS health endpoint or variety monitoring docs. |
| **Missing sections** | No troubleshooting section. No "I ran `kata start` and nothing happened" debug flow. No explanation of what the kata bundle manifest.yaml actually contains (link to registry path). |

**Score: 7.5/10** — Strongest guide in corpus. Missing technical failure recovery and kanban bridge.

### 1.3 `lora-adapter-store-guide.md`

| Dimension | Assessment |
|-----------|-----------|
| **First command clear?** | No — the guide starts with type definitions (`TrainedLoRAAdapter` struct). No quick start. No "here's the first thing you type." |
| **What success looks like?** | No explicit success signals. The user has to infer from API semantics. |
| **Failure modes?** | Error types enumerated but no recovery workflows. What happens when `store_adapter` fails with DuplicateId? What about provider timeout? |
| **Examples with expected output?** | Code snippets show Rust API calls but no terminal output, no "after running this you'll see..." statements. |
| **Cross-references?** | None to lora-training-guide.md (the natural upstream). None to CNS observability docs for span interpretation. |
| **Missing sections** | Quick Start. End-to-end workflow (train → store → route → deploy). Troubleshooting. CLI commands (guide is pure Rust API reference). Link to lora-training-guide. |

**Score: 3/10** — Reads like API reference, not a user guide. A new user cannot accomplish a task from this alone.

### 1.4 `lora-training-guide.md`

| Dimension | Assessment |
|-----------|-----------|
| **First command clear?** | Partial — starts with dataset concepts, not a quick-start command. |
| **What success looks like?** | Yes — "By the end, you will have a deployed LoRA adapter serving inference with cost tracking." |
| **Failure modes?** | DatasetPipeline validation errors are mentioned but no recovery workflows. No "training job failed" debugging. No "adapter deployment failed" section. |
| **Examples with expected output?** | Good — Rust code snippets include expected output comments (`// Output: Ingested 150 records in ChatML format`). Missing: CLI command examples for the training MCP tools. |
| **Cross-references?** | References `training-decomposition-traces.md`. No link to lora-adapter-store-guide.md (the downstream consumer). No link to skill-user-guide for using trained skills. |
| **Missing sections** | CLI command examples (not just Rust API). Deployment verification (CNS span confirmation). Link to adapter store guide. Budget/cost estimation before training. |

**Score: 5.5/10** — Good technical depth, weak on user-facing CLI workflow and guide chain.

### 1.5 `REPLICANT-ONBOARDING-WALKTHROUGH.md`

| Dimension | Assessment |
|-----------|-----------|
| **First command clear?** | Yes — `kask` starts onboarding. Prerequisites listed with verify commands. |
| **What success looks like?** | Yes — expected output shown in §4 (replicant list, sovereignty status, CNS health). |
| **Failure modes?** | Troubleshooting § covers: daemon unavailable, replicant not authenticated, model not found, permission denied. |
| **Examples with expected output?** | Yes — verification output shown. Chat session example with prompt formatting. |
| **Cross-references?** | Next Steps § links to AGENT-POD-CREATION, COMMON-AGENT-PATTERNS, OPERATIONS_RUNBOOK, kata-user-guide. Reference § links to magna-carta, PRINCIPLES, MDS, REPL spec, AgentService spec. |
| **Missing sections** | No Matrix/cloud deployment link. No "I want to use a cloud model instead of Ollama" quick switch. Status is "Draft" — should be promoted. |

**Score: 8.5/10** — Best guide in corpus. Clear, actionable, has troubleshooting, has expected output. Should be the template for other guides.

### 1.6 `AGENT-POD-CREATION-GUIDE.md`

| Dimension | Assessment |
|-----------|-----------|
| **First command clear?** | Yes — structured 8-step process. Step 1 is "Requirements Discovery." |
| **What success looks like?** | Implicit — the pod activates, CNS spans emit. No explicit "verification checklist" or "your pod is working if..." |
| **Failure modes?** | Troubleshooting § covers: pod creation fails, registration fails, activation fails, CNS span emission fails, visibility/access errors. |
| **Examples with expected output?** | Persona YAML templates are comprehensive. Missing: CLI output after `kask pod activate`. Missing: what `kask pod list` looks like after creation. |
| **Cross-references?** | API Endpoints § links internally. Missing: link to COMMON-AGENT-PATTERNS ("choose a pattern from..."). Missing: link to kanban-user-guide for task assignment to pods. Missing: link to skill-user-guide for capability definition. |
| **Missing sections** | Verification output examples. "Quick validation checklist" after each step. Budget estimation for pod operations. |

**Score: 7/10** — Comprehensive structure, good troubleshooting, missing success verification signals.

### 1.7 `COMMON-AGENT-PATTERNS.md`

| Dimension | Assessment |
|-----------|-----------|
| **First command clear?** | No — starts with taxonomy table, then dives into Specialist Bot personas. No "pick a pattern and run this command." |
| **What success looks like?** | No success criteria per pattern. |
| **Failure modes?** | No troubleshooting per pattern. No "what if my bridge agent can't reach the external workspace." |
| **Examples with expected output?** | Persona/dispatch YAML is comprehensive. No CLI output showing pattern deployment. |
| **Cross-references?** | Template Library § references template types. Missing: link to AGENT-POD-CREATION for the creation workflow. Missing: link to ACP-ZED-CONFIGURATION for replicant patterns in Zed. |
| **Missing sections** | Quick Start (pick pattern → deploy → verify). Per-pattern troubleshooting. Per-pattern verification. Link to pod creation guide. |

**Score: 4/10** — Good reference catalog, not a usable guide. A new user can't go from zero to deployed pattern.

### 1.8 `COMPANIES-GUIDE.md`

| Dimension | Assessment |
|-----------|-----------|
| **First command clear?** | Yes — setup with `kask keystore set`, then test commands. |
| **What success looks like?** | Implicit — tool returns data. Sample workflow § shows end-to-end with 8 numbered steps. |
| **Failure modes?** | Limitations § covers scope but not: API key expired, rate limit hit, provider routing failure. No troubleshooting section. |
| **Examples with expected output?** | Good — sample workflow with commentary. CSV format example. Missing: actual tool output examples for key tools. |
| **Cross-references?** | Related Documents § links to PROJECT_STATUS, mcp-server-roadmap, PRINCIPLES. Missing: link to wallet docs (for rJoule cost tracking). |
| **Missing sections** | Troubleshooting (API key errors, provider fallback failures). Expected output for common queries. Cost model explanation. |

**Score: 6.5/10** — Good capability overview, well-structured workflow example, weak on error recovery.

### 1.9 `ACP-ZED-CONFIGURATION.md`

| Dimension | Assessment |
|-----------|-----------|
| **First command clear?** | Yes — 3-step quick start with actual commands. |
| **What success looks like?** | Implicit — agent appears in Zed's agent picker. |
| **Failure modes?** | Troubleshooting § covers: agent not appearing, replicant not authenticated, not assigned to acp role, startup gates failed, no inference output. |
| **Examples with expected output?** | Configuration JSON is complete. Missing: screenshot or text output of successful agent panel integration. |
| **Cross-references?** | Missing: link to REPLICANT-ONBOARDING for initial setup. Missing: link to models API for model selection. |
| **Missing sections** | Verification after setup (what to check). Multi-replicant configuration example. Environment variable reference for all providers. |

**Score: 7.5/10** — Clear, actionable, good troubleshooting. Needs upstream guide links.

### 1.10 `skill-user-guide.md`

| Dimension | Assessment |
|-----------|-----------|
| **First command clear?** | Yes — `kask skill list` is the first command. |
| **What success looks like?** | Implicit — skill appears in list, bundle runs. |
| **Failure modes?** | Troubleshooting § covers 5 common issues with symptoms, causes, and fixes. Excellent. |
| **Examples with expected output?** | Yes — `kask skill list` shows expected table output. `kask skill show` documented. |
| **Cross-references?** | Links to skill-designer-guide, dual-layer model, PRINCIPLES, AGENTS.md. |
| **Missing sections** | No "end-to-end workflow" (discover → activate → verify → use in chat). No explanation of what happens when a bundle fails to compose. |

**Score: 8/10** — Strong guide. Troubleshooting table is the gold standard for the corpus.

### 1.11 `skill-designer-guide.md`

| Dimension | Assessment |
|-----------|-----------|
| **First command clear?** | No — starts with dual-layer model explanation. No "create your first skill in 5 minutes" quick start. |
| **What success looks like?** | Lifecycle Checklist § covers "from zero to production" steps. No "your skill is working if..." verification. |
| **Failure modes?** | Common Pitfalls § covers 8 items. No runtime failure troubleshooting (template render error, CNS span not found). |
| **Examples with expected output?** | YAML templates are comprehensive. Missing: "after registration, run `kask skill list` and you'll see..." |
| **Cross-references?** | Links to skill-user-guide, dual-layer model, PRINCIPLES, AGENTS.md. Missing: link to kata-user-guide for kata-type skills. |
| **Missing sections** | Quick Start (create minimal skill → register → verify). CLI verification commands. Skill testing workflow. |

**Score: 6/10** — Good reference for experienced skill authors, intimidating for newcomers.

---

## 2. Cross-Guide Integration Gaps

### 2.1 Critical: No Getting-Started Flow

No guide answers: "I just installed hKask. What do I do in order?"

The path should be:
```
REPLICANT-ONBOARDING → ACP-ZED-CONFIGURATION → kanban (create board, first task) 
→ kata (starter on task) → CNS health check → skill discovery → agent pod creation
```

Currently each guide is an island. The Replicant Onboarding guide has a "Next Steps" section that links to some guides, but there is no unified journey.

### 2.2 Kanban→Kata→CNS Bridge Missing

- `kanban-user-guide.md` § "Katas on Tasks" shows `/kanban coach`, `/kanban improve`, `/kanban practice` commands but doesn't explain that these invoke the kata system.
- `kata-user-guide.md` never mentions kanban. The two systems are conceptually linked (kanban tasks are the "gemba" where kata is practiced) but no guide explains this.
- Neither guide links to CNS health monitoring as the verification step after kata practice.

### 2.3 Training→Adapter Store Chain Broken

- `lora-training-guide.md` describes training and deployment but doesn't link to `lora-adapter-store-guide.md` for storage/routing.
- `lora-adapter-store-guide.md` doesn't link to `lora-training-guide.md` as the upstream producer.
- Neither links to `skill-user-guide.md` for using the trained adapter as a skill.

### 2.4 Pod Creation→Patterns→Skills Chain Weak

- `AGENT-POD-CREATION-GUIDE.md` has Common Agent Patterns § duplicated from `COMMON-AGENT-PATTERNS.md` but doesn't link to it.
- Neither guide links to `skill-user-guide.md` for defining capabilities as skills.
- Neither links to `COMPANIES-GUIDE.md` for domain-specific agent examples.

---

## 3. Prioritized Fix List

### Priority 1 — Critical (blocks new users)

| # | Issue | Guide(s) | Fix |
|---|-------|----------|-----|
| 1 | **No error recovery in kanban guide** | `kanban-user-guide.md` | Add Troubleshooting section covering: board creation failure, invalid template, malformed JSON from LLM, populate failure, permission denied on assignment, de-jammer false positives. |
| 2 | **No getting-started flow** | All | Create `docs/user-guides/GETTING-STARTED.md` as a unified journey: onboard → configure Zed → create kanban board → run first kata → check CNS → discover skills. Each step links to the detailed guide. |
| 3 | **Kanban→Kata→CNS bridge missing** | `kanban-user-guide.md`, `kata-user-guide.md` | Add cross-reference sections explaining how kanban tasks are the gemba for kata practice, and how CNS health verifies the improvement cycle. |

### Priority 2 — High (confuses users mid-journey)

| # | Issue | Guide(s) | Fix |
|---|-------|----------|-----|
| 4 | **No Quick Start in lora-adapter-store-guide** | `lora-adapter-store-guide.md` | Add Quick Start section with first CLI command, expected output, and link to lora-training-guide as upstream. |
| 5 | **No troubleshooting in kata-user-guide** | `kata-user-guide.md` | Add Troubleshooting section: template render failures, CNS span missing, consent denied, model unavailable, automaticity not advancing. |
| 6 | **COMMON-AGENT-PATTERNS missing deploy→verify flow** | `COMMON-AGENT-PATTERNS.md` | Add per-pattern Quick Start (deploy command → verification output). Add per-pattern troubleshooting. |
| 7 | **lora-training-guide missing CLI examples** | `lora-training-guide.md` | Add CLI command examples alongside Rust API snippets. Add link to lora-adapter-store-guide. |

### Priority 3 — Medium (quality of life)

| # | Issue | Guide(s) | Fix |
|---|-------|----------|-----|
| 8 | **Expected output missing from kanban guide** | `kanban-user-guide.md` | Add example terminal output for key commands: board create, view, move, note, deliver, verify. |
| 9 | **Expected output missing from kata guide** | `kata-user-guide.md` | Add example terminal output for starter, improvement, and coaching runs. |
| 10 | **AGENT-POD-CREATION missing verification** | `AGENT-POD-CREATION-GUIDE.md` | Add "Verification Checklist" after each step. Add expected output for `kask pod list` and `kask cns spans`. |
| 11 | **skill-designer-guide missing Quick Start** | `skill-designer-guide.md` | Add "Create Your First Skill in 5 Minutes" section with a minimal working example end-to-end. |
| 12 | **COMPANIES-GUIDE missing troubleshooting** | `COMPANIES-GUIDE.md` | Add Troubleshooting: API key expired, rate limited, provider routing failure, symbol not found. |
| 13 | **REPLICANT-ONBOARDING "Draft" status** | `REPLICANT-ONBOARDING-WALKTHROUGH.md` | Promote to "Active" — it's the best guide. Add cloud model quick-switch. |
| 14 | **Cross-guide link audit** | All | Systematic check: every guide should link to its upstream (what do I need first?), downstream (what do I do next?), and related (what else is relevant?). |

---

## 4. Summary Statistics

| Metric | Count |
|--------|-------|
| Guides audited | 11 |
| Critical issues (P1) | 3 |
| High issues (P2) | 4 |
| Medium issues (P3) | 7 |
| Total fix items | 14 |
| Best guide | REPLICANT-ONBOARDING-WALKTHROUGH (8.5/10) |
| Weakest guide | lora-adapter-store-guide (3/10) |
| Most common gap | Missing expected output / terminal examples (6 guides) |
| Second most common gap | Missing or weak troubleshooting (6 guides) |
| Structural gap | No unified getting-started flow connecting all guides |

---

## 5. Recommendation

1. **Immediately:** Create `GETTING-STARTED.md` that chains the existing guides into a linear journey. This single document resolves P1 issues #2 and #3 by providing the narrative glue.

2. **This week:** Add troubleshooting sections to kanban and kata guides (P1 #1, P2 #5). Add Quick Start to lora-adapter-store-guide (P2 #4).

3. **Next sprint:** Add expected output examples and verification checklists across all guides (P3 #8–11). Run the cross-guide link audit (P3 #14).

4. **Template for future guides:** The REPLICANT-ONBOARDING-WALKTHROUGH should be the canonical template. Required sections: Prerequisites, Quick Start (first command), What Success Looks Like, Expected Output, Troubleshooting, Cross-References.
