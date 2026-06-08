# HANDOFF — hLexicon Expansion + Skill Cross-References + Doc Updates

## 1. Session Context

Previous session recomposed all 7 translated skills for hKask's dual-layer architecture and fixed the systematic `template_type: Cognition` → `KnowAct` bug across all manifests and .j2 files. Three remaining tasks were identified. Work began but was not completed due to token limits.

## 2. What Was Verified This Session

- All 4 recomposed SKILL.md files (skill-translator, skill-discovery, skill-maintenance, skill-manager) confirmed correct — dual-layer aware, registry-as-primary, proper template_type/visibility/energy_cap validation
- skill-translator-translate.j2 confirmed updated to produce dual-layer output (manifest_yaml + templates[] + skill_md)
- skill-translator-analyze.j2 confirmed updated with 3-way step classification (cognitive/workflow/guardrail) with registry+SKILL.md targets
- No remaining `template_type: Cognition` in any manifest.yaml or .j2 `[inference]` frontmatter (verified via grep)
- Comment-only `Cognition` references remain in legacy .j2 files (knowact/*.j2, curator/*.j2) using pre-standard `# Cognition —` format — these are benign, not parsed by `TemplateType::parse_str()`

## 3. Remaining Tasks

### Task 1: Add 34 missing hLexicon terms to the canonical source

**Status: NOT STARTED**

34 terms used in skill manifests are not registered in the workspace hLexicon. The canonical source is `docs/architecture/reference/hKask-hLexicon.md`. After editing the markdown, regenerate the YAML: `cargo test -p hkask-templates regenerate_workspace_yaml -- --ignored`

**Missing terms by domain (proposed classification):**

**KnowAct (20 terms):**
- `amplify` — Increase regulatory or response variety
- `analyze` — Decompose into components for understanding
- `attenuate` — Reduce system or disturbance variety
- `compress` — Distill and reduce context volume
- `deepen` — Extract a smaller interface from a shallow module (already in bootstrap but not workspace YAML)
- `design` — Define structure and interfaces of a component
- `explore` — Search systematically for patterns or friction
- `fix` — Apply a corrective change to resolve a defect
- `instrument` — Add targeted observation points to a code path
- `isolate` — Separate a concern or variable from its context
- `map` — Produce a structured representation of component relationships
- `observe` — Watch and record system behavior without intervention
- `plan` — Define ordered steps toward a goal before execution
- `predict` — Forecast an outcome from current evidence
- `rank` — Order items by a comparative criterion
- `resolve` — Determine a winner or course from competing alternatives
- `score` — Assign a numeric assessment to an artifact
- `synthesize` — Combine disparate elements into a coherent whole
- `trace` — Follow a causal or provenance chain (already in bootstrap but not workspace YAML)
- `translate` — Convert from one representation or format to another

**FlowDef (8 terms):**
- `defer` — Postpone action pending future evidence
- `deprecate` — Mark for future removal while remaining functional
- `enforce` — Ensure a constraint or rule is obeyed
- `install` — Place an artifact into its operational location
- `list` — Enumerate items in a collection
- `prune` — Remove an artifact from the corpus
- `retire` — Permanently remove a deprecated artifact
- `search` — Look for candidates across available sources

**WordAct (6 terms):**
- `extract` — Pull specific structured data from a source
- `gap` — Identify an uncovered requirement or missing capability
- `reproduce` — Re-create a bug or behavior from a known procedure
- `substitute` — Replace a term or reference with an equivalent
- `validate` — Confirm an artifact meets defined criteria
- `write` — Produce or persist content

**How to add them:**
1. Edit `docs/architecture/reference/hKask-hLexicon.md` — add new subsections to each domain (e.g., §3.6 "Diagnosis Cognition", §3.7 "Skill Management Cognition" under KnowAct; §2.8 "Skill Lifecycle" under FlowDef; §1.7 "Diagnostic Acts" under WordAct)
2. Update the term count headers in the Contents table and section titles
3. Update the hLexicon Term Index section (alphabetical)
4. Update the "87 terms" references to the new total
5. Run `cargo test -p hkask-templates regenerate_workspace_yaml -- --ignored`
6. Verify: `cargo test -p hkask-templates` should pass (including `hlexicon_yaml_matches_markdown`)
7. Verify: re-run the cross-reference script to confirm 0 missing terms

**Cross-reference script (for verification):**
```python
python3 -c "
import yaml, glob
all_terms = set()
for f in sorted(glob.glob('registry/templates/*/manifest.yaml')):
    with open(f) as fh:
        data = yaml.safe_load(fh)
    if data and 'hlexicon_terms' in data:
        for t in data['hlexicon_terms']:
            all_terms.add(t)
    if data and 'templates' in data:
        for tmpl in data['templates']:
            if 'lexicon_terms' in tmpl:
                for t in tmpl['lexicon_terms']:
                    all_terms.add(t)
with open('registry/registries/hlexicon-workspace.yaml') as fh:
    ws = yaml.safe_load(fh)
ws_terms = set()
for cat in ws.get('hlexicon', {}):
    for entry in ws['hlexicon'][cat]:
        ws_terms.add(entry['term'])
missing = sorted(all_terms - ws_terms)
print(f'Missing from workspace: {len(missing)}')
if missing:
    print(f'Terms: {missing}')
"
```

### Task 2: Add "Registry Templates" sections to original 8 skills

**Status: NOT STARTED**

Add a "Registry Templates" section to each of these SKILL.md files, matching the format already added to the 3 conceptual skills:

| Skill | Registry Dir | Templates |
|-------|-------------|-----------|
| `diagnose` | `registry/templates/diagnose/` | diagnose-loop.j2 (KnowAct), diagnose-hypothesise.j2 (KnowAct), diagnose-instrument.j2 (KnowAct), diagnose-fix.j2 (KnowAct) |
| `grill-me` | `registry/templates/grill-me/` | grill-me-round.j2 (KnowAct), grill-me-assess.j2 (KnowAct), grill-me-escalate.j2 (KnowAct) |
| `tdd` | `registry/templates/tdd/` | tdd-plan.j2 (KnowAct), tdd-tracer.j2 (KnowAct), tdd-refactor.j2 (KnowAct), tdd-verify.j2 (KnowAct), tdd-gap-check.j2 (KnowAct) |
| `coding-guidelines` | `registry/templates/coding-guidelines/` | guidelines-assess.j2 (KnowAct), guidelines-apply.j2 (KnowAct), guidelines-verify.j2 (KnowAct) |
| `zoom-out` | `registry/templates/zoom-out/` | zoom-out-context.j2 (KnowAct) |
| `handoff` | `registry/templates/handoff/` | handoff-compact.j2 (KnowAct), handoff-artifacts.j2 (KnowAct), handoff-skills-suggest.j2 (KnowAct), handoff-compose.j2 (WordAct) |
| `improve-codebase-architecture` | `registry/templates/improve-codebase-architecture/` | arch-explore.j2 (KnowAct), arch-candidates.j2 (KnowAct), arch-deepen.j2 (KnowAct) |
| `magna-carta-verifier` | `registry/templates/magna-carta-verifier/` | (need to check what templates exist) |

**Format template:**
```markdown
## Registry Templates

This skill's runtime templates live in `registry/templates/<name>/`:

| Template | Type | Purpose |
|----------|------|--------|
| `<name>-<step>.j2` | KnowAct/WordAct/FlowDef | <concise purpose> |

The SKILL.md (this file) teaches the Zed coding agent the <domain> methodology. The .j2 templates are executable process steps the hKask runtime invokes during `kask chat` sessions.
```

Insert this section **before** the "When to Use" section in each SKILL.md.

**Note:** `magna-carta-verifier` may not have registry templates yet. Check `registry/templates/magna-carta-verifier/` before writing the section. If no templates exist, add the section noting the registry layer is not yet present (flag as incomplete per dual-layer model).

### Task 3: Update DDMVSS §12.2 and test-program §11 descriptions

**Status: NOT STARTED**

The recomposed skill-management SKILL.md files have updated descriptions. Check if the descriptions in these two docs still match:

1. `docs/architecture/DDMVSS.md` §12.2 (Skill References table) — currently has:
   - `skill-translator` — "Translate skills between format systems" → should be "Translate agent skills into hKask's dual-layer architecture (registry crate + SKILL.md companion)"
   - `skill-discovery` — "Find, evaluate, and install skills from external sources" → should be "Find, evaluate, and install dual-layer skills (SKILL.md + registry templates)"
   - `skill-maintenance` — "Audit skill corpus for staleness, coverage gaps, quality" → should be "Audit hKask's dual-layer skill architecture for staleness, coverage gaps, and quality degradation"
   - `skill-manager` — "CRUD meta-skill for skill corpus management" → should be "Dual-layer CRUD for the skill corpus across Zed agent and registry layers"

2. `docs/specifications/test-program.md` §11 (Skill-to-DDMVSS Mapping) — may have similar descriptions that need updating

**Process:** Read both sections, update descriptions to match the recomposed SKILL.md frontmatter `description` fields.

## 4. Key Decisions

1. **template_type: Cognition is INVALID** — only WordAct/KnowAct/FlowDef are parseable. All fixed.
2. **Registry crate is the primary runtime artifact** — SKILL.md is companion guide.
3. **34 missing hLexicon terms** are proposed additions, not errors in the manifests. They need to be registered in the canonical markdown source and the workspace YAML regenerated.
4. **`deepen` and `trace`** are in the Rust bootstrap hLexicon (`hkask-types/src/lexicon.rs`) but NOT in the workspace YAML. They were likely added after the last YAML regeneration. Adding them to the markdown and regenerating will fix this.
5. **The hLexicon markdown is the single source of truth.** Never edit the YAML directly. Edit the markdown, then regenerate.

## 5. Verification Commands

After all tasks complete:

```bash
# Verify no invalid template_type remains
grep -r 'template_type: Cognition' registry/templates/ --include='*.j2'
grep -r 'type: Cognition' registry/templates/ --include='manifest.yaml'

# Verify hLexicon coverage (should show 0 missing)
python3 -c "
import yaml, glob
all_terms = set()
for f in sorted(glob.glob('registry/templates/*/manifest.yaml')):
    with open(f) as fh: data = yaml.safe_load(fh)
    if data and 'hlexicon_terms' in data:
        for t in data['hlexicon_terms']: all_terms.add(t)
    if data and 'templates' in data:
        for tmpl in data['templates']:
            if 'lexicon_terms' in tmpl:
                for t in tmpl['lexicon_terms']: all_terms.add(t)
with open('registry/registries/hlexicon-workspace.yaml') as fh: ws = yaml.safe_load(fh)
ws_terms = set()
for cat in ws.get('hlexicon', {}):
    for entry in ws['hlexicon'][cat]: ws_terms.add(entry['term'])
missing = sorted(all_terms - ws_terms)
print(f'Missing from workspace: {len(missing)} {missing if missing else \"\"}')
"

# Verify builds
cargo check -p hkask-types
cargo check -p hkask-templates
cargo test -p hkask-templates
```

---
*Handoff composed: 2026-06-07 — hLexicon Expansion + Skill Cross-References + Doc Updates*