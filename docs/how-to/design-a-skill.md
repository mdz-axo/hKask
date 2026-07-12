---
title: "How to Design a Skill — How-To Guide"
audience: [operators, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# How to Design a Skill

**Goal:** Create a new PDCA skill from scratch — writing the manifest, templates, testing locally, and publishing to the registry.

hKask skills are iterative PDCA (Plan-Do-Check-Act) loops that compose multiple Jinja2 templates into autonomous search, learning, and implementation cycles. Skills are discovered from a two-zone model: `.agents/skills/` (private source) and `skills/` (public export surface).

---

## 1. Skill Anatomy

A skill consists of three layers:

```
.agents/skills/my-skill/               ← Private zone (source of truth)
├── SKILL.md                            ← YAML front matter + markdown body

registry/templates/my-skill/            ← Registry layer (canonical per P5.1)
├── manifest.yaml                       ← FlowDef manifest
└── *.j2                                ← Jinja2 templates
```

- **`SKILL.md`** has YAML front matter (`name`, `visibility`, `namespace`, `description`) and a markdown body describing the skill. It's a generated companion for development tooling.
- **`manifest.yaml`** declares the template set and, for FlowDef skills, convergence criteria and energy budget.
- **`*.j2` files** are Jinja2 templates rendered at invocation time with context variables.

### Skill Domain Types

Skills fall into three domains, inferred from their registry manifest:

| Domain | Template Type | Behavior |
|--------|--------------|----------|
| **FlowDef** | `FlowDef` | PDCA cycle with convergence threshold, energy budget, and loop action — the skill iterates autonomously |
| **KnowAct** | `KnowAct` | Knowledge-action template — provides reasoning guidance (one-shot) |
| **WordAct** | `WordAct` | Word-action template — provides language-level guidance (one-shot) |

FlowDef is the most powerful: it runs until it converges on a quality threshold, exhausts its energy budget, or escalates to the Curator.

---

## 2. Writing a `manifest.yaml`

Create `registry/templates/my-skill/manifest.yaml`:

```yaml
# Manifest for my-skill
templates:
  - path: plan.j2
    type: FlowDef
  - path: do.j2
    type: FlowDef
  - path: check.j2
    type: FlowDef
  - path: act.j2
    type: FlowDef

# FlowDef-specific fields
convergence:
  threshold: 0.05        # Output delta must drop below this to converge
  max_iterations: 10     # Hard cap on PDCA iterations

gas:
  cap: 500               # Maximum energy budget in abstract units
  per_iteration: 50      # Energy reserved per iteration

loop:
  action: "If not converged, iterate with refined context"
```

### Convergence Threshold

The convergence threshold controls when the PDCA cycle stops. After each iteration, the output delta (difference from the previous iteration) is measured. If delta < threshold, the skill has converged and returns its output.

- **Threshold = 0.0**: converge on zero delta (exact match to previous output)
- **Threshold = 0.05**: converge when outputs are 95%+ similar (typical for reasoning skills)
- **Threshold = 0.10+**: converge earlier (use for skills where refinement has diminishing returns)

### Gas Budget

Gas represents the skill's energy budget. Each iteration reserves a portion of the budget. The PDCA cycle stops when gas is exhausted.

- **`gas.cap`**: Total budget for the skill execution
- **`gas.per_iteration`**: Reserved per iteration; unused gas is refunded (hold-settle pattern)

### Loop Action

The loop action is a natural-language instruction injected between PDCA iterations. It guides the LLM on how to refine its approach based on the previous output.

---

## 3. Writing Templates (`.j2` Files)

Templates are Jinja2 files rendered with context variables at invocation time. The standard PDCA structure:

### `plan.j2` — Plan phase

```jinja2
You are executing the "my-skill" skill. This is the PLAN phase.

Context: {{ context }}

Based on the context above, develop a structured plan for achieving the goal.
Consider:
1. What information is needed
2. What tools should be used
3. What intermediate outputs are required

Return your plan as a numbered list.
```

### `do.j2` — Do phase

```jinja2
You are executing the "my-skill" skill. This is the DO phase.

Plan:
{{ plan_output }}

Context: {{ context }}

Execute the plan above. Use the available tools. Report your results.
```

### `check.j2` — Check phase

```jinja2
You are executing the "my-skill" skill. This is the CHECK phase.

Expected outcome: {{ plan_output }}
Actual outcome: {{ do_output }}

Evaluate:
1. Did the execution match the plan?
2. Are the results complete and correct?
3. What gaps or errors exist?

Return a concise assessment.
```

### `act.j2` — Act phase

```jinja2
You are executing the "my-skill" skill. This is the ACT phase.

Assessment: {{ check_output }}

Based on the assessment:
1. If issues were found, propose corrective actions
2. If complete, produce the final output
3. If stuck, request escalation to the Curator

Return your action.
```

### Context Variables

The following context variables are automatically available during template rendering:

| Variable | Source | Description |
|----------|--------|-------------|
| `{{ context }}` | User-supplied | The original invocation context (query, prompt, parameters) |
| `{{ plan_output }}` | Previous PDCA phase | Output from the Plan phase |
| `{{ do_output }}` | Previous PDCA phase | Output from the Do phase |
| `{{ check_output }}` | Previous PDCA phase | Output from the Check phase |
| `{{ iteration }}` | FlowDef engine | Current PDCA iteration number |

---

## 4. Writing the `SKILL.md`

Create `.agents/skills/my-skill/SKILL.md`:

```markdown
---
name: my-skill
visibility: private
namespace: my-namespace
description: A custom PDCA skill for automated code review
---

# My Skill

This skill performs an automated code review using a PDCA cycle:
- **Plan:** Analyze the code structure and identify review targets
- **Do:** Execute the review using available tools
- **Check:** Validate findings against quality criteria
- **Act:** Produce a review report with recommendations

## Invocation

Provide the file path or code snippet as context:

```
/skill my-skill "Review src/main.rs"
```
```

The `visibility` field controls where the skill appears:

| Value | Zone | Description |
|-------|------|-------------|
| `private` | `.agents/skills/` only | Author's working copy; not exported |
| `public` | Both zones | Available to all; published to `skills/` |
| `shared` | Both zones | Available to authenticated replicants |

---

## 5. Testing a Skill Locally

### Step 1: Verify the skill is discovered

```bash
kask skill list
```

Your skill should appear in the private zone:

```
  private zone (.agents/skills/):
    my-skill                    visibility=private  namespace=my-namespace  hash=abc123def456
```

### Step 2: Check skill status

```bash
kask skill status my-skill
```

Output:

```
Skill: my-skill
  Private zone: .agents/skills/my-skill
  Visibility:   private
  Source hash:  abc123def456
  Public zone:  (not published)
  Status:       private (not exported)
```

### Step 3: Audit the skill's health

```bash
kask skill audit
```

Output shows registry manifest integrity, template file existence, and content hash consistency:

```
Skill audit report (fail_below=0.00)

skill                          score         status active defects
my-skill                        1.00         active    yes 0
coding-guidelines               1.00         active    yes 0
diagnose                        0.85         active    yes 1
    - Template "analyze.j2" referenced in manifest but not found
```

### Step 4: Invoke from the REPL

Start the REPL and invoke your skill:

```bash
kask chat
```

Inside the REPL, skills are loaded at startup from both zones. Use:

```
/skill my-skill "Review the authentication module in src/auth.rs"
```

The REPL routes this through the `hkask-mcp-skill` MCP server:

1. **Lookup** — Skill ID resolved against the loaded registry
2. **Template rendering** — Jinja2 templates rendered with context variables
3. **System prompt prepended** — Tool-awareness preamble added automatically
4. **Inference** — Rendered template sent to inference port (`temperature: 0.3`, `max_tokens: 2048`)
5. **CNS span** — `cns.tool.skill_execute` emitted

---

## 6. Publishing to the Registry

To make a private skill available in the public zone:

```bash
kask skill publish my-skill
```

This:
1. Copies the skill directory from `.agents/skills/my-skill/` to `skills/<namespace>--my-skill/`
2. Sets `visibility: public` in the published copy's `SKILL.md`
3. Sets `namespace` to the current replicant name (from `HKASK_REPLICANT_NAME`, git `user.name`, or `"local"`)
4. Emits a `cns.skill` span (`skill_published`)

Output:

```
Published 'my-skill' as 'my-namespace--my-skill' to public zone: skills/my-namespace--my-skill
  Sortable by replicant: my-namespace
  Sortable by skill:    my-skill
```

After publishing, verify:

```bash
kask skill status my-skill
```

Output:

```
Skill: my-skill
  Private zone: .agents/skills/my-skill
  Visibility:   public
  Source hash:  abc123def456
  Public zone:  skills/my-namespace--my-skill
  Published by: my-namespace
  Public hash:  abc123def456
  Status:       in sync
```

---

## 7. Skill Polarity and Zones

### Two-Zone Model

```
.agents/skills/              (private zone — source of truth)
├── my-skill/
│   └── SKILL.md
└── ...

skills/                      (public zone — export surface)
├── my-namespace--my-skill/
│   └── SKILL.md
└── ...

registry/templates/          (registry layer — canonical source per P5.1)
├── my-skill/
│   ├── manifest.yaml
│   └── *.j2
└── ...
```

**P5.1 Rule:** The registry crate (`manifest.yaml` + `*.j2`) is the canonical source. `SKILL.md` is a generated companion. When they disagree, the registry is authoritative.

### Visibility Rules

- A skill in the **private zone** may have any visibility
- A skill in the **public zone** MUST have `visibility: public` or `visibility: shared`
- Zone-visibility mismatches emit a warning but do not block registration — the `visibility` field wins

---

## 8. Common Mistakes and Debugging

### Manifest Not Found

**Symptom:** `kask skill list` shows the skill but `kask skill audit` reports no manifest.

**Cause:** Missing `registry/templates/my-skill/manifest.yaml`.

**Fix:** Create the manifest file. Without it, the skill domain defaults to `KnowAct` (reasoning companion).

### Template Path Mismatch

**Symptom:** Audit shows "Template 'X.j2' referenced in manifest but not found."

**Cause:** The manifest references a `.j2` file that doesn't exist in `registry/templates/my-skill/`.

**Fix:** Either create the missing template or remove the reference from `manifest.yaml`.

### Skill Not Found in REPL

**Symptom:** `/skill my-skill` says "Skill 'my-skill' not found."

**Cause:** The REPL was started from a directory without `.agents/skills/` or the skill directory doesn't contain `SKILL.md`.

**Fix:** Start the REPL from the project root. Ensure `.agents/skills/my-skill/SKILL.md` exists. Skills are loaded at REPL startup, not hot-reloaded.

### Zone-Visibility Mismatch Warning

**Symptom:** Warning at REPL startup: "Skill 'my-skill' is in the public zone but declares visibility: private."

**Cause:** The skill was moved to `skills/` but `visibility: private` is still set in `SKILL.md`.

**Fix:** Either move the skill back to `.agents/skills/` or change `visibility: public` in the `SKILL.md` front matter.

### Template Rendering Failure

**Symptom:** `skill_execute` returns "Template render error."

**Cause:** Jinja2 syntax error in one of the `.j2` files, or a referenced context variable is misspelled.

**Fix:** Validate Jinja2 syntax in all `.j2` files. Ensure context variable names (`{{ context }}`, `{{ plan_output }}`, etc.) match exactly.

### Inference Port Not Wired

**Symptom:** `skill_execute` returns "Inference failed."

**Cause:** The `hkask-mcp-skill` MCP server was started without an inference port.

**Fix:** The REPL wires the inference port automatically when started via `kask chat`. Standalone mode requires explicit configuration — set `HKASK_DEFAULT_PROVIDER` and the corresponding `XX_API_KEY`.

### Published Skill Has Stale Content

**Symptom:** `kask skill status my-skill` shows "local changes since last publish."

**Fix:** Run `kask skill publish my-skill` to update the public zone copy.

### Content Hash Mismatch

**Symptom:** Audit reports hash mismatch between private and public zones.

**Cause:** The `SKILL.md` files differ between `.agents/skills/` and `skills/`.

**Fix:** Re-publish: `kask skill publish my-skill`.

---

## Related

- [Invoke a Skill](invoke-a-skill.md) — Install, activate, and invoke skills
- [Compose Skills](compose-skills.md) — Bundle multiple skills with cascade ordering
- [Bootstrap an MCP Server](bootstrap-mcp-server.md) — Create an MCP server for skill tooling
