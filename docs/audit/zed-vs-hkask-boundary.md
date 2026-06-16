# Task 0 — Zed vs. hKask Skill-Loading Boundary Verification

**Date:** 2026-06-16  
**Classification:** Evidence (direct code observation)

---

## Finding

| System | Skill artifact | What loads it | What executes it | Runtime behavior |
|--------|----------------|---------------|----------------|------------------|
| **Zed editor** | `.agents/skills/<name>/SKILL.md` | Zed agent system prompt / `skill` tool / `/name` slash command | Injected as XML-escaped text into the model conversation | LLM receives instructions and acts on them in conversation |
| **hKask runtime** | `registry/templates/<name>/manifest.yaml` + `*.j2` | `SkillLoader` registers `.agents/skills/*/SKILL.md` metadata; `Registry`/`SqliteRegistry` store `RegistryEntry` templates; `ManifestExecutor` executes `BundleManifest` YAML | `ManifestExecutor` renders `.j2` templates via minijinja and dispatches MCP tools per YAML steps | Actual process cascade: select → populate → execute |

**Conclusion:** In hKask, the Zed layer (`.agents/skills/*`) is a **companion guide for the coding agent only**. It is **not** the runtime behavior source. Runtime behavior is driven by the registry layer (`registry/templates/*`) loaded by `SkillLoader`/`Registry`/`SqliteRegistry` and executed by `ManifestExecutor`.

---

## Evidence by File

### 1. `crates/hkask-types/src/lexicon.rs`

```rust
pub enum TemplateType {
    WordAct,   // Jinja2 prompts — .j2
    KnowAct,   // Jinja2 cognition — .j2
    FlowDef,   // YAML process manifests — .yaml
}
```

- Line 17-35: `TemplateType` enum comment explicitly maps types to file formats.
- Line 64-70: `file_extension()` returns `"yaml"` for `FlowDef`, `"j2"` for `WordAct`/`KnowAct`.
- Line 86-92: `infer_from_extension("j2")` returns `Some(KnowAct)`; `.j2` cannot be `FlowDef` by extension.

### 2. `crates/hkask-templates/src/registry.rs`

```rust
pub struct Registry {
    templates: HashMap<String, RegistryEntry>,
    skills: HashMap<String, Skill>,
    bundles: HashMap<String, hkask_types::BundleManifest>,
    ...
}
```

- Line 1-10: Module header states FlowDef is YAML process manifests.
- Line 18-29: `Registry` stores `RegistryEntry` (templates), `Skill`, and `BundleManifest`.
- Line 337-357: `bootstrap()` loads `registry/templates/bootstrap-registry.yaml` (a flat list of `RegistryEntry` records), not `SKILL.md` body text.

### 3. `crates/hkask-templates/src/executor.rs`

```rust
pub async fn execute_manifest(
    &self,
    manifest: &BundleManifest,
    initial_context: HashMap<String, Value>,
) -> Result<HashMap<String, Value>> {
    ...
    context = self.execute_step(step, context).await?;
}
```

- Line 1-26: `ManifestExecutor` executes `BundleManifest` cascades with `select`/`populate`/`execute` steps.
- Line 18-22: `select`/`populate` steps render `.j2` templates via minijinja relative to `registry/templates`.
- Line 185-200: `execute` steps invoke MCP tools.

### 4. `crates/hkask-templates/src/skill_loader.rs`

```rust
fn load_skill(&self, skill_dir: &Path, zone: SkillZone) -> Result<Skill, String> {
    let skill_md_path = skill_dir.join("SKILL.md");
    let content = fs::read_to_string(&skill_md_path)?;
    let front_matter = Self::parse_front_matter(&content)?;
    ...
    let mut skill = Skill::new(&id, TemplateType::FlowDef)
        .with_visibility(visibility)
        .with_zone(zone);
    skill.compute_content_hash();
    Ok(skill)
}
```

- Line 1-11: `SkillLoader` scans `.agents/skills/` and `skills/` for `SKILL.md` files.
- Line 132-184: `load_skill()` reads only YAML frontmatter (`name`, `visibility`, `namespace`, `description`). It does **not** parse the SKILL.md body, extract steps, or link to registry `.j2`/`.yaml` files.
- Line 173: Default domain is `TemplateType::FlowDef`, but this is just registration metadata, not an executable manifest.

---

## Implications for the Audit

1. **A `.j2` file declaring `template_type: FlowDef` is a type-system violation.** The runtime type says FlowDef = `.yaml`; `.j2` = WordAct or KnowAct.
2. **A skill without a registry layer is incomplete.** `SKILL.md` alone gives the coding agent guidance but provides no executable templates for the hKask runtime.
3. **Bundles must be YAML `BundleManifest` files** (or entries in `registry/manifests/` / SQLite) that reference calibrated primary skills, not additional `.j2` wrappers.
4. **The prior agents' mistake:** They authored DDMVSS-style templates (`Cognition`, `Prompt`, `Process`) as `.j2` files and called them `FlowDef`, conflating the Zed prompt-injection model with the hKask registry/executor model.
