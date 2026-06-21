---
name: bug-hunt
version: "0.30.0"
visibility: Public
namespace: bug-hunt
description: >
  Bug hunting skill. Runs expeditions against target crates to find threats
  to user-defined quality. Applies Weinberg's quality definition, Beizer's
  bug taxonomy, Bach's Heuristic Test Strategy Model, and Hendrickson's
  exploratory testing charters. Uses MCP tools (file read, code search,
  terminal) to probe code and produces structured bug reports.
trigger: >
  User says "hunt bugs in X", "find bugs", "bug hunt", "explore for bugs",
  "what bugs are in this crate", or specifies a target with quality criteria.
---

# Bug Hunt Skill

A bug hunting skill that explores target crates for threats to user-defined quality.

## When to Use

- "Hunt bugs in hkask-wallet"
- "Find bugs in hkask-cns — quality criteria: no energy budget violations"
- "Explore hkask-types for data boundary bugs"
- "What bugs exist in hkask-capability?"

## How It Works

1. **Charter:** Generates a focused exploration mission using Hendrickson format
2. **Probe:** Reads code, searches for bug patterns, runs tests via MCP tools
3. **Oracle:** Evaluates findings against user-defined quality criteria (Weinberg)
4. **Taxonomize:** Classifies bugs into Beizer taxonomy with severity
5. **Report:** Produces structured JSON bug report with fix suggestions

## Input

- `target`: crate name, module, or function to hunt in
- `quality_criteria`: what "quality" means for this target (Weinberg: value to some person who matters)

## Output

JSON report with findings, classifications, confidence scores, and pattern signatures.

## Composition

Reasoning patterns from TDD (contract verification), diagnose (systematic investigation), grill-me (verdict challenging), adversarial-red-team (attack probes), and kata (PDCA learning) are embedded as prompt instructions in the expedition template.

## Registry

- **Canonical source:** `registry/templates/bug-hunt/manifest.yaml`
- **Template:** `registry/templates/bug-hunt/bug-hunt-expedition.j2`
