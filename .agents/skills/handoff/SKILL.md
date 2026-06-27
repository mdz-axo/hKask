---
name: handoff
visibility: public
description: "Continuation prompt for session handoffs. Captures what was done, what remains, key decisions, and recommended next steps so a new agent session can pick up where the previous one left off. Use when starting a new session that needs to continue work from a prior session, or when creating a handoff document before ending a session."
---

# Handoff Skill

Produce a structured handoff document that lets a fresh agent pick up exactly where the current session left off — no more, no less.

## Registry Templates

This skill's runtime templates live in `registry/templates/handoff/`:

| Template | Type | Purpose |
|----------|------|--------|
| `handoff-compact.j2` | KnowAct | Compress session context into a structured summary |
| `handoff-artifacts.j2` | KnowAct | Identify and catalog artifacts by path or URL reference |
| `handoff-skills-suggest.j2` | KnowAct | Suggest relevant skills for the next agent session |
| `handoff-compose.j2` | WordAct | Assemble final handoff document with redaction |

The SKILL.md (this file) teaches the Zed coding agent the handoff methodology. The .j2 templates are executable process steps the hKask runtime invokes during `kask chat` sessions.

## When to Use

- Ending a session with unfinished work
- Starting a new session that must continue prior work
- The user says "handoff", "continue from where we left off", or "create a continuation prompt"

## Output Structure

Every handoff document must contain these sections, in order:

### 1. Session Context

One paragraph: what this session was trying to accomplish and how far it got (e.g., "90% complete", "blocked on X").

### 2. What Was Done

Accomplishments only — not the journey. Group by logical area (documentation, code, tests, etc.). For each:
- State what changed, not how you got there
- Reference files by path, never duplicate their contents
- Note compilation/lint/test status ("all compile cleanly", "2 tests failing")

### 3. What Remains

Ordered by priority (HIGH / MEDIUM / LOW). For each item:
- What specifically needs to happen
- Where in the codebase the work should happen
- Any dependencies or blockers
- Recommended strategy or approach

### 4. Recommended Skills and Tools

Which skills the next agent should invoke and why. Include specific commands (e.g., `cargo check -p <crate>`, `cargo clippy -p <crate> -- -D warnings`).

### 5. Key Decisions to Preserve

Numbered list of architectural or design decisions made during this session, with rationale. These are decisions that a future agent must not silently reverse without understanding why they were made.

## Output Destination

Write the handoff document to `docs/handoffs/` with the filename format:

```
[brief-description]-[YYYY-MM-DD].md
```

**Examples:**
- `docs/handoffs/wlt-cns-ph5-2026-06-12.md`
- `docs/handoffs/repl-srvr-2026-06-12.md`
- `docs/handoffs/cndsr-cmpr-2026-06-12.md`

The brief description should be ≤12 characters, using abbreviations where needed (e.g., `wlt` for wallet, `cns` for CNS, `ph5` for phase 5, `srvr` for server, `cndsr` for condenser, `cmpr` for compression).
The date is the session date in ISO 8601 format.

If the user specifies a different path, use that instead.

Do NOT write the handoff document inside any skill directory (`.agents/skills/*/`). Skill directories contain only skill definitions — they are not output targets.

## Rules

1. **Never write files inside this skill's directory.** The `.agents/skills/handoff/` directory contains only the skill definition (SKILL.md). Handoff documents are session artifacts — write them to `docs/handoffs/[brief-description]-[date].md` or to a user-specified path. Writing inside the skill directory clutters it and risks overwriting SKILL.md, which destroys the skill for future agents.
2. **Reference, don't duplicate.** Files, PRDs, ADRs, specs — point to them by path. The next agent can read them.
3. **Progress, not process.** "Removed KillZoneDetector from runtime.rs" not "First I opened runtime.rs, then I deleted lines 45-60, then I..."
4. **Decisions carry rationale.** Every decision must include *why* and what alternatives were considered.
5. **No sensitive data.** No API keys, tokens, passwords, or PII. Redact if present.
6. **Current state is precise.** Exactly where things left off, including what's unfinished, what compiles, what doesn't.
7. **Max 8000 tokens.** If the handoff exceeds this, the session was too broad — narrow the scope.

## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/handoff.yaml`

### PDCA Convergence
- **Threshold:** 0.05 (converged when metric ≤ this)
- **Improvement ratio:** 0.05 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = handoff packet is complete, actionable, and redaction-safe

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 3 rJ
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
