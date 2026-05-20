---
title: "hKask Migration Strategy"
audience: [maintainers, sysadmins, migration leads]
last_updated: 2026-05-20
togaf_phase: "F"
version: "1.0.0"
status: "Active"
domain: "Cross-cutting"
---

<!-- TOGAF_DOMAIN: Cross-cutting -->
<!-- VERSION: 1.0.0 -->
<!-- STATUS: Active -->
<!-- LAST_UPDATED: 2026-05-20 -->

# hKask Migration Strategy

**Purpose:** Terminology migration, document lifecycle management, and Git-based architecture repository governance.

**Related:** [`roadmap.md`](roadmap.md), [`GOVERNANCE.md`](../standards/GOVERNANCE.md)  
**TOGAF Phase:** F — Migration Planning[^togaf-f]

---

## 1. Executive Summary

hKask migration strategy addresses terminology alignment (νKask→CNS, OKH→cns.*), document lifecycle (Draft→Active→Deprecated→Removed), and Git-based architecture repository governance.

**Migration Scope:**
- **Terminology:** 3 deprecated terms → CNS-aligned replacements
- **Documents:** 2 files for archival (vKask-*.md)
- **Code:** Already updated (CNS terminology in place)
- **Git History:** Preserved as architecture repository

**Timeline:**
- **v0.21.0:** Code updated, documentation overhaul complete
- **v0.22.0:** Archive superseded documents, finalize migration
- **v1.0:** All migration complete, terminology consistent

---

## 2. Terminology Migration

### 2.1 Deprecated → Current Mapping

| Deprecated Term | Replacement | Context | Status |
|-----------------|-------------|---------|--------|
| **νKask** | CNS (Cybernetic Nervous System) | System monitoring, telemetry | ⚠️ Docs pending archival |
| **OKH spans** | `cns.*` spans | Tracing namespace | ✅ Complete |
| **Three registries** | Unified registry | Template storage | ✅ Complete |
| **Feedback crate** | CNS spans | Feedback handling | ✅ Complete |
| **ν-event** | ν-event (retained) | Cybernetic event structure | ✅ Retained (correct) |
| **ℏKask** | hKask (retained) | Project name | ✅ Retained (correct) |

### 2.2 Migration Commands

```bash
# Check for deprecated terminology in code
grep -r "νKask\|OKH\|three registries" crates/ --include="*.rs"
# Expected: 0 occurrences (code already updated)

# Check for deprecated terminology in docs
grep -r "νKask\|OKH\|three registries" docs/ --include="*.md" --exclude-dir=archive
# Expected: 52 occurrences in vKask-*.md files only

# Verify CNS terminology in code
grep -r "cns\." crates/ --include="*.rs" | head -10
# Expected: Multiple occurrences (cns.tool.*, cns.prompt.*, etc.)
```

### 2.3 Migration Verification

**Code Verification:**
```bash
# CNS namespace usage
grep -r "Cns\|cns" crates/hkask-cns/src/ --include="*.rs" | wc -l
# Expected: >100 occurrences

# Capability attenuation
grep -r "attenuate\|attenuation" crates/hkask-ensemble/src/ --include="*.rs" | wc -l
# Expected: >20 occurrences
```

**Documentation Verification:**
```bash
# Count deprecated vs current terminology
deprecated=$(grep -r "νKask\|OKH" docs/ --include="*.md" --exclude-dir=archive | wc -l)
current=$(grep -r "CNS\|cns\." docs/ --include="*.md" | wc -l)
echo "Deprecated: $deprecated, Current: $current"
# Expected after archival: Deprecated ≈ 0, Current > 100
```

---

## 3. Document Lifecycle Migration

### 3.1 Superseded Documents for Archival

| File | Reason | Replacement | Action |
|------|--------|-------------|--------|
| `vKask-cybernetic-constant.md` | Deprecated terminology (νKask, OKH) | `security-architecture.md`, `data-architecture.md` | `git rm` |
| `vKask-erd.md` | Deprecated terminology (νKask, OKH) | `hKask-erd.md`, `data-architecture.md` | `git rm` |

### 3.2 Archival Process

```bash
# Step 1: Verify replacements exist
ls docs/architecture/security-architecture.md
ls docs/architecture/data-architecture.md
ls docs/architecture/hKask-erd.md

# Step 2: Archive superseded documents
git rm docs/architecture/vKask-cybernetic-constant.md
git rm docs/architecture/vKask-erd.md

# Step 3: Update cross-references (if any)
grep -r "vKask" docs/ --include="*.md"
# Update any remaining references to point to new documents

# Step 4: Commit archival
git commit -m "Archive superseded vKask documents (replaced by CNS architecture)

- vKask-cybernetic-constant.md → security-architecture.md, data-architecture.md
- vKask-erd.md → hKask-erd.md, data-architecture.md

Migration: νKask→CNS terminology complete.
TOGAF Phase: F — Migration Planning"
```

### 3.3 Document Metadata Migration

**Files Requiring Metadata Headers (18 files):**

| File | Missing Fields | Priority |
|------|----------------|----------|
| `hKask-architecture-index.md` | Version, TOGAF Phase, Domain | Medium |
| `hKask-Curator-persona.md` | Version, TOGAF Phase, Domain | Medium |
| `hKask-hLexicon.md` | Version, TOGAF Phase, Domain | Medium |
| `okapi-capability-model.md` | TOGAF Phase, Domain | Low |
| `pragmatic-composition-erd.md` | Version, TOGAF Phase, Domain | Medium |
| `future_work_resolved.md` | Version, TOGAF Phase, Domain | Low |
| `registry-deferred-work.md` | Version, TOGAF Phase, Domain | Low |
| `OPEN_QUESTIONS.md` | TOGAF Phase, Domain | Low |

**Migration Script:**
```bash
#!/bin/bash
# Add metadata header to document
add_metadata() {
  file="$1"
  version="$2"
  phase="$3"
  domain="$4"
  
  # Insert after first line (title)
  sed -i "2i\\
**Version:** $version\\
**Last-Updated:** 2026-05-20\\
**Status:** Active\\
**Audience:** [architects, developers]\\
**TOGAF Phase:** $phase\\
**Domain:** $domain" "$file"
}
```

---

## 4. Git History as Architecture Repository

### 4.1 Recovery Procedures

**Recover Deleted Document:**
```bash
# Find commit that deleted the file
git log --diff-filter=D -- docs/architecture/vKask-cybernetic-constant.md

# Show the file at the commit before deletion
git show <sha>^:docs/architecture/vKask-cybernetic-constant.md

# Restore to working directory
git checkout <sha>^ -- docs/architecture/vKask-cybernetic-constant.md
```

**View Document History:**
```bash
# Full history of a document
git log --follow docs/architecture/hKask-architecture-master.md

# Show changes between versions
git diff <sha1> <sha2> -- docs/architecture/hKask-erd.md
```

### 4.2 Architecture Repository Governance

**Principles:**[^arch-repo]
1. **Git is canonical** — No separate archive system
2. **History is immutable** — Deleted files recoverable via `git log`
3. **Active tree is current** — Only active documents linked from index
4. **No archived links** — Active docs must not link to `archive/` directory

**Verification:**
```bash
# Check for links to archive directory
grep -r "archive/" docs/ --include="*.md"
# Expected: 0 occurrences (active docs must not link to archive)

# Verify all internal links resolve
# (Manual check or implement link checker)
```

---

## 5. Migration Risks & Mitigation

### 5.1 Risk Matrix

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| **Broken links after archival** | Medium | Low | Update cross-references before archival |
| **Lost institutional knowledge** | Low | Medium | Git history preserves all content |
| **Terminology confusion during transition** | Medium | Low | Clear mapping table (§2.1), update all docs atomically |
| **Code/doc divergence** | Low | High | CI/CD check for deprecated terms |

### 5.2 Rollback Procedures

**If Migration Fails:**
```bash
# Rollback document archival
git revert <archival-commit-sha>

# Restore deprecated terminology (if needed)
# Not recommended — terminology migration is one-way
```

---

## 6. Migration Completion Criteria

### 6.1 v0.22.0 Milestone

| Criterion | Verification | Status |
|-----------|--------------|--------|
| **Code terminology** | Zero `νKask`, `OKH` in `crates/` | ✅ Complete |
| **Document archival** | `git rm vKask-*.md` | ⏳ Pending (Task 7) |
| **Cross-reference update** | Zero broken links | ⏳ Pending |
| **Metadata headers** | All 18 files updated | ⏳ Deferred |
| **Citation density** | All architecture docs have citations | ⏳ Deferred |

### 6.2 v1.0 Release Milestone

| Criterion | Verification | Target |
|-----------|--------------|--------|
| **TOGAF coverage** | 9 phases documented | 100% |
| **Writing Excellence** | ≥80% documents pass | ≥80% |
| **Diagram alignment** | 100% Mermaid blocks aligned | 100% |
| **Terminology consistency** | Zero deprecated terms | 100% |

---

## 7. References

[^togaf-f]: The Open Group. (2011). *TOGAF Standard, Version 9.1*. Phase F: Migration Planning. <https://pubs.opengroup.org/architecture/togaf9-doc/arch/chap17.html>.
[^arch-repo]: hKask Project. (2026). *GOVERNANCE.md*. §5: Document Lifecycle.

---

*This migration strategy is effective 2026-05-20. Execute archival (Task 7) before v0.22.0 release.*

**Next:** Task 7 — Archive vKask files.
