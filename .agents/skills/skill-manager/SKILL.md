---
name: skill-manager
visibility: public
description: "Meta-skill for CRUD operations on the skill corpus. List, validate, build, install, and prune skills from within a session. Use when the user says 'list skills', 'validate skills', 'create a skill', 'install a skill', or 'prune skills'. Pairs with skill-discovery, skill-maintenance, and skill-translator."
---

# Skill Manager

Manage the skill corpus from within a session. List skills, validate their format, scaffold new ones, install from external sources, and prune deprecated ones. This is the CRUD layer — `skill-discovery` handles finding candidates, `skill-maintenance` handles auditing, `skill-translator` handles format conversion, and this skill handles the operations.

## Skill Directory

All project-local skills live in `.agents/skills/`. Global skills live in `~/.agents/skills/`. Prefer project-local unless the skill is a personal workflow the user wants everywhere.

## Operations

### List Skills

Read `.agents/skills/` and report:

For each skill directory:
1. Read `SKILL.md` frontmatter
2. Report: name, description (truncated to 80 chars), visibility

Output format:
```
Skills (N total):
  [name]  [visibility]  [description excerpt]
  ...
```

If a skill directory has no `SKILL.md` or the frontmatter is invalid, flag it.

### Validate Skills

For each skill (or a specific skill):

```
Format checks:
  ☐ SKILL.md exists in directory
  ☐ Frontmatter has `name` field
  ☐ `name` matches directory name exactly
  ☐ `name` is lowercase-hyphenated (^[a-z0-9]+(-[a-z0-9]+)*$)
  ☐ `description` field is present
  ☐ `description` is 1–1024 characters
  ☐ `description` is specific and actionable (not generic)
  ☐ Body is non-empty

Quality checks:
  ☐ Instructions use imperative voice ("Do X" not "Consider X")
  ☐ No placeholder content (TODO, FIXME, "fill in later")
  ☐ No contradictions within the skill
  ☐ Skill scope is clear (does one thing well)

Safety checks:
  ☐ No Magna Carta violations in instructions
  ☐ No headless constraint violations (no visual UI references)
  ☐ No `todo!` / `unimplemented!` / `#[deprecated]` references
  ☐ If CNS spans referenced, they exist in canonical namespace
  ☐ If crate paths referenced, they exist in workspace
```

Report: total skills, pass/fail counts, specific failures with fix suggestions.

### Build a Skill

When the user wants to create a new skill:

1. **Confirm scope**: Project-local (`.agents/skills/`) or global (`~/.agents/skills/`)
2. **Choose name**: Confirm with user. Must be lowercase-hyphenated, descriptive.
3. **Create directory**: `.agents/skills/<name>/`
4. **Write SKILL.md**: Frontmatter (name, description) + body with instructions
5. **Add supporting files** if needed: templates, examples, reference docs
6. **Validate**: Run the validation checks above
7. **Confirm**: Show the user the created skill and ask for review

Follow `create-skill` conventions for SKILL.md format.

### Install a Skill

When installing from an external source:

1. **Source is hKask-format**: Copy directory to `.agents/skills/<name>/`, validate
2. **Source is different format**: Use `skill-translator` to convert, then install
3. **Validate** after installation
4. **Verify**: The skill should be discoverable by description matching

### Prune a Skill

When removing a skill:

**Soft prune** (deprecate without deleting):
1. Add `disable-model-invocation: true` to SKILL.md frontmatter
2. Skill remains on disk but is not auto-loaded

**Hard prune** (delete):
1. Confirm with user — this is irreversible
2. Delete the skill directory from `.agents/skills/`
3. Note: if the skill was version-controlled, it can be recovered from git

### Stats

Report skill corpus statistics:
- Total skills (project-local + global)
- Skills by visibility (public vs private)
- Average description length
- Coverage assessment (how many common task patterns have matching skills)
- Staleness summary (quick check: any obvious broken references?)

## Decision Guide

| User Request | Operation |
|-------------|-----------|
| "List skills" / "Show skills" | List |
| "Validate skills" / "Check skills" | Validate |
| "Create a skill" / "New skill" | Build |
| "Install a skill" / "Add a skill" | Install |
| "Remove a skill" / "Delete a skill" | Prune (confirm with user) |
| "Deprecate a skill" | Soft prune |
| "Skill stats" / "How many skills" | Stats |
| "Find a skill for X" | Delegate to `skill-discovery` |
| "Is this skill stale?" | Delegate to `skill-maintenance` |
| "Translate this skill" | Delegate to `skill-translator` |
| "Bundle skills" | Delegate to `skill-bundler` |

## Safety

- **Never** delete a skill without user confirmation
- **Never** modify a skill's instructions without telling the user what changed
- **Always** validate after build or install
- **Always** back up (git tracks changes — commit before major modifications)

## When to Use This Skill

- **"List skills" / "Show skills"**: Report the skill corpus
- **"Validate skills"**: Check format, quality, and safety of all skills
- **"Create a skill"**: Scaffold a new SKILL.md
- **"Install a skill"**: Add an external skill to the corpus
- **"Remove a skill"**: Prune or deprecate
- **"Skill stats"**: Quick corpus health overview