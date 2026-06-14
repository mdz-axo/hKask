---
title: "Handoff Lifecycle Policy"
audience: [project maintainers, agents, replicants]
last_updated: 2026-06-14
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [lifecycle, curation]
---

# Handoff Lifecycle Policy

**Purpose:** Defines the lifecycle of transient session handoffs in `docs/handoffs/`, their transition to archive, and the governing principles for handoff hygiene.

**Governing Principles:** P5 (Pared Surface), P6 (No Dead Docs)

---

## 1. What Is a Handoff?

A handoff is a transient document recording implementation state at the end of an agent session. It exists to allow continuation by another agent session (or the same agent after context reset). Handoffs are:

- **Descriptive, not prescriptive.** They record what was done, what remains, and key decisions — not future plans.
- **Supersession-oriented.** Each handoff should reference its predecessor if it continues the same workstream.
- **Temporary.** Handoffs are committed to git history and cleaned from the working tree when superseded.

---

## 2. Handoff Lifecycle States

```
Created → Active → Superseded → Archived (git history)
                    ↓
                  Stale (>30d no successor) → Archived
```

### 2.1 Created

A handoff is created at the end of an agent session. It includes:

- Date (ISO 8601) in filename: `{topic}-YYYY-MM-DD.md`
- What was accomplished
- What remains
- Key architectural decisions made
- Reference to predecessor handoff (if continuing a workstream)
- Recommended next steps

### 2.2 Active

A handoff is active while its workstream is ongoing. Active handoffs live in `docs/handoffs/` and are visible in the working tree. An active handoff:

- Has a clear successor path (work is continuing)
- Has been created within the last 30 days
- Contains state not yet encoded elsewhere (code, ADRs, specs)

### 2.3 Superseded

A handoff is superseded when a newer handoff in the same workstream explicitly carries forward its essential state. The successor must:

- Reference the superseded handoff by filename
- Re-encode all essential architectural decisions
- State: "This session builds on {predecessor}, which carried state X, Y, Z"

Upon supersession, the handoff is removed from the working tree via `git rm` and committed. Git history is the archive of record.

### 2.4 Stale

A handoff becomes stale when:

- No successor handoff exists in the same workstream
- The handoff is older than 30 days
- OR the workstream is demonstrably complete (code, ADRs, and specs exist)

Stale handoffs follow the same archival procedure as superseded ones: `git rm` from working tree.

---

## 3. Archive Procedure

1. Verify the handoff is superseded (has a successor that carries forward state) or stale (>30 days, no active workstream).
2. Remove from working tree: `git rm docs/handoffs/{filename}`
3. Commit with message: `docs: archive handoff {filename} (superseded by {successor})` or `docs: archive stale handoff {filename}`
4. No on-disk archive copy is kept. Git history is the canonical archive.

The `docs/archive/MANIFEST.md` records archive decisions but does not store document contents. Handoffs are never moved to `docs/archive/` — that directory is for non-handoff documents being retired.

---

## 4. Handoff Hygiene Rules

| Rule | Enforcement |
|------|-------------|
| No handoff stays in working tree >30 days without a successor | Manual review; CI flag (future) |
| Every handoff must reference its predecessor (if continuing workstream) | Manual review |
| Superseded handoffs are `git rm`'d, not moved to archive/ | Policy; verifiable via `git log -- docs/handoffs/` |
| Handoffs never contain forward-looking plans — plans live in `docs/plans/` | Manual review |
| No YAML frontmatter required (handoffs are transient, not formal docs) | Deliberate exclusion — they are not indexed in portal |

---

## 5. Relationship to Other Lifecycle Policies

- **`DOCUMENTATION_STANDARDS.md`** governs formal documents with frontmatter. Handoffs are exempt from frontmatter requirements due to their transient nature.
- **`MDS_SCAFFOLD.md`** governs document placement. Handoffs live in `docs/handoffs/` only.
- **`docs/archive/MANIFEST.md`** records retired non-handoff documents. Handoff archival is tracked in git commit history.
- **`docs/plans/`** contains forward-looking work. Handoffs that drift into planning should be rewritten as plan documents.

---

## 6. Verification

```bash
# Count active handoffs in working tree
ls docs/handoffs/*.md 2>/dev/null | wc -l

# View handoff history
git --no-pager log --oneline -- docs/handoffs/

# Check for handoffs older than 30 days
find docs/handoffs -name "*.md" -mtime +30 2>/dev/null

# Verify no handoffs in archive/
ls docs/archive/handoffs/ 2>/dev/null && echo "VIOLATION: Handoffs in archive/ directory" || echo "OK"
```

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
