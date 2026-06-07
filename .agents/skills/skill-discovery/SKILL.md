---
name: skill-discovery
visibility: public
description: "Find, evaluate, and install agent skills from external sources. Detect capability gaps, search for candidate skills, validate quality, and guide installation. Use when the user says 'find a skill for X', 'I need a skill that does Y', or when a task pattern has no matching skill."
---

# Skill Discovery

Manage the full lifecycle of finding, evaluating, and installing skills for the hKask agent. When no existing skill covers a task pattern, detect the gap, search for candidates, evaluate them, and guide installation.

## The Skill Lifecycle

```
Gap detected → Search for skill → Evaluate candidate → Validate quality
  → Guide installation → Verify loaded → Ready
```

Most failures happen at validation. Know the rules and enforce them before installing.

## Detecting Capability Gaps

When should you suggest finding or building a new skill?

| Trigger | Gap Type | What to Look For |
|---------|----------|-----------------|
| Task pattern has no matching skill description | Coverage gap | Search `.agents/skills/` for matching descriptions |
| User says "I wish the agent could do X" | Feature gap | Is X within hKask's scope? (No visual UI, no external monitoring stacks) |
| Repeated manual steps across sessions | Automation gap | Can those steps be codified into a skill? |
| User asks about a domain outside current skills | Knowledge gap | A knowledge-only skill may help |
| A constraint is repeatedly violated with no skill to guide resolution | Governance gap | A skill enforcing that constraint may help |
| Current skill instructions are vague or incomplete | Quality gap | Improve the existing skill rather than find a new one |

When you spot a gap, say so explicitly: "No skill currently covers X. Want me to search for one, or should I help build one?"

## Searching for Skills

### Where skills live

- GitHub repositories (`skills/` or `.agents/skills/` directories)
- Direct URLs to SKILL.md files
- The user's local filesystem
- hKask registry (if skills are registered in the template registry)

### Search strategies

Use web search to find skills:

```
# General pattern
search query: "hKask skill [domain] site:github.com"
search query: "zed agent skill [domain] .agents/skills"
search query: "SKILL.md [domain] agent instructions"

# Specific examples
"hKask skill cybernetics site:github.com"
"SKILL.md agent skill debugging methodology"
"zed .agents/skills code review"
```

### What to search when results are thin

If no hKask-specific skill exists:
1. Search for the underlying capability: "[topic] agent instructions methodology"
2. Search for general-purpose skill patterns that could be adapted
3. Search for the domain knowledge itself, then consider building a skill from scratch

A general-purpose methodology can be adapted into a hKask skill using `skill-translator`.

## Evaluating a Candidate Skill

When you find a candidate skill, evaluate it against these criteria before suggesting installation:

### Format Validation

```
☐ File exists: .agents/skills/<name>/SKILL.md
☐ Frontmatter has `name` field matching directory name
☐ `name` is lowercase-hyphenated (^[a-z0-9]+(-[a-z0-9]+)*$)
☐ `description` field is present, 1–1024 chars, specific and actionable
☐ Body is non-empty and contains concrete instructions
☐ No references to files or paths that don't exist within the skill directory
```

If any check fails, report specifically which one and how to fix it.

### Quality Evaluation

```
☐ Instructions are concrete — "Do X" not "Consider doing X"
☐ Instructions are self-contained — no assumed context not provided
☐ No placeholder content ("TODO", "Fill in later")
☐ No contradictions between instructions
☐ Skill scope is clear — it does one thing well, not many things poorly
☐ No unnecessary complexity — instructions match the problem's difficulty
```

### Safety Evaluation

```
☐ No Magna Carta violations (episodic memory exposure without consent, bypass OCAP)
☐ No headless constraint violations (no visual UI, no Grafana, no Prometheus)
☐ No `todo!` / `unimplemented!` / `#[deprecated]` references (P6/P7 compliance)
☐ If skill references CNS spans, they exist in the canonical namespace list
☐ If skill references crate paths, they exist in the workspace
```

### Red Flags

- Vague descriptions ("Helps with code") — reject, request specificity
- External dependencies not in workspace — flag for review
- Network calls without documentation — flag
- Instructions that assume a specific model or persona — needs adaptation
- Skills that embed secrets or API keys — reject immediately

## Installation Guide

When the user asks to install a skill:

### Quick install (existing hKask-format skill)

1. Copy the skill directory to `.agents/skills/<name>/`
2. Verify: read `SKILL.md` frontmatter — `name` matches directory, description is present
3. The skill is auto-discovered by the Zed agent on the next conversation

### Install with translation (non-hKask format)

Use `skill-translator` to convert the source skill to hKask format, then install as above.

### Verify after installation

1. **Format check**: Read the SKILL.md — frontmatter validates
2. **Content check**: Instructions are concrete and actionable
3. **Safety check**: No Magna Carta violations, no headless constraint violations
4. **CNS check**: If the skill references `cns.*` spans, they exist
5. **Integration check**: The skill's description is specific enough to be matched when relevant

If any step fails, identify the failure mode:
- Format fails → fix frontmatter (name, description)
- Content fails → rewrite vague instructions
- Safety fails → report the violation and refuse installation
- CNS fails → update span references
- Integration fails → improve description specificity

## Building a Skill from Scratch

When no skill exists and the user wants you to help create one, use `create-skill` conventions:

1. **Decide scope**: Project-local (`.agents/skills/`) vs global (`~/.agents/skills/`)
2. **Choose name**: Descriptive, lowercase-hyphenated
3. **Write SKILL.md**: Frontmatter (name, description) + imperative instructions
4. **Validate**: Run the quality and safety evaluations above
5. **Optionally add supporting files**: Templates, examples, reference docs

### Knowledge skill template

A knowledge skill teaches the agent something without prescribing actions:

```markdown
---
name: topic-name
description: "Specific description of what the agent learns and when to use it."
---

# Topic Name

[Imperative instructions for what the agent should know and do with this knowledge]
```

### Procedural skill template

A procedural skill gives the agent a step-by-step process:

```markdown
---
name: procedure-name
description: "Specific description of the procedure and when to use it."
---

# Procedure Name

[Step-by-step instructions with concrete actions, tools, and expected outcomes]
```

## Skill Hygiene

### When to update a skill

- Architecture changes make references stale (moved crates, renamed spans)
- New Magna Carta principles or constraints added
- Instructions are too vague or too rigid for real use
- The skill's description no longer matches its instructions

### When to retire a skill

- The domain it covers is no longer relevant
- A better skill supersedes it
- The skill's instructions are consistently ignored (wrong abstraction level)
- The user says they don't need it

Retirement: set `disable-model-invocation: true` in frontmatter (soft deprecation) or delete the skill directory (hard retirement).

### Sharing skills

Skills are self-contained directories. Share by:
1. Push the `.agents/skills/<name>/` directory to a git repo
2. The recipient copies it into their own `.agents/skills/` directory

## When to Use This Skill

- **"Find a skill for X":** Detect gap → search → evaluate → install
- **"I need the agent to do Y":** Check if a skill exists; if not, search or build
- **"Can I use this skill from [other project]?":** Evaluate format compatibility, translate if needed
- **Recurring manual patterns:** Suggest codifying into a skill
- **After installing a skill:** Verify it loaded correctly