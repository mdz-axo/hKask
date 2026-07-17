//! Skill command handlers for `kask skill`
//!
//! Implements CLI display logic for skill visibility management.
//! Two-zone model: `.agents/skills/` (source) → `skills/` (export surface).

use crate::cli::SkillAction;
use crate::commands;
use hkask_ports::{InferencePort, SkillZone};
use hkask_services_skill as skill;
use hkask_services_skill::audit::{SkillAuditor, SkillStatus};
use hkask_templates::{Registry, SkillLoader};
use hkask_types::template::LLMParameters;
use hkask_types::visibility::Visibility;
use serde_json::json;
use std::path::PathBuf;

/// Default project root (current directory).
fn project_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Run a skill command.
///
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  action is valid
/// post: skill command executed
pub fn run_skill(action: SkillAction) {
    match action {
        SkillAction::List { visibility } => {
            list_skills(visibility.as_deref());
        }
        SkillAction::Status { name } => {
            skill_status(&name);
        }
        SkillAction::Publish { name } => {
            skill_publish(&name);
        }
        SkillAction::Audit { fail_below, json } => {
            skill_audit(fail_below, json);
        }
        SkillAction::Derive { name } => {
            skill_derive(&name);
        }
    }
}

/// List skills, optionally filtered by visibility.
fn list_skills(visibility_filter: Option<&str>) {
    let root = project_root();
    let vis_filter = visibility_filter.and_then(Visibility::parse_str);

    for zone in [SkillZone::Private, SkillZone::Public] {
        let zone_dir = root.join(zone.directory());
        if !zone_dir.exists() {
            continue;
        }

        let skill_infos = match skill::discover_skills(&zone_dir) {
            Ok(dirs) => dirs,
            Err(e) => {
                eprintln!("Error scanning {}: {}", zone_dir.display(), e);
                continue;
            }
        };

        if skill_infos.is_empty() {
            continue;
        }

        println!("  {} zone ({}):", zone.as_str(), zone_dir.display());

        for info in &skill_infos {
            // Apply filter
            if let Some(filter) = vis_filter
                && info.visibility != filter
            {
                continue;
            }

            let hash_display = info
                .content_hash
                .as_deref()
                .map(|h| &h[..h.len().min(12)])
                .unwrap_or("-");
            let ns_display = info.namespace.as_deref().unwrap_or("-");

            println!(
                "    {:30} visibility={:8} namespace={:12} hash={}",
                info.name,
                info.visibility.as_str(),
                ns_display,
                hash_display
            );
        }
    }
}

/// Show skill status — compare private source vs published copy.
fn skill_status(name: &str) {
    let root = project_root();
    let private_dir = root.join(SkillZone::Private.directory()).join(name);

    let public_dir = skill::find_public_skill(&root, name);

    if !private_dir.exists() {
        eprintln!("Skill '{}' not found in private zone.", name);
        return;
    }

    let private_vis = skill::read_skill_visibility(&private_dir.join("SKILL.md"));
    let private_hash = skill::compute_file_hash(&private_dir.join("SKILL.md"));
    let private_ns = skill::read_skill_namespace(&private_dir.join("SKILL.md"));

    println!("Skill: {}", name);
    println!("  Private zone: {}", private_dir.display());
    println!("  Visibility:   {}", private_vis.as_str());
    if let Some(ref ns) = private_ns {
        println!("  Namespace:    {}", ns);
    }
    println!(
        "  Source hash:  {}",
        private_hash.as_deref().unwrap_or("(error)")
    );

    if let Some(ref pub_dir) = public_dir {
        let public_hash = skill::compute_file_hash(&pub_dir.join("SKILL.md"));
        let pub_namespace = skill::read_skill_namespace(&pub_dir.join("SKILL.md"));
        println!("  Public zone:  {}", pub_dir.display());
        if let Some(ref ns) = pub_namespace {
            println!("  Published by: {}", ns);
        }
        println!(
            "  Public hash:  {}",
            public_hash.as_deref().unwrap_or("(error)")
        );

        match (private_hash, public_hash) {
            (Some(ph), Some(pubh)) if ph == pubh => {
                println!("  Status:       in sync");
            }
            (Some(_), Some(_)) => {
                println!(
                    "  Status:       local changes since last publish — run `kask skill publish {}` to update",
                    name
                );
            }
            _ => {
                println!("  Status:       unable to compare hashes");
            }
        }
    } else {
        println!("  Public zone:  (not published)");
        if private_vis == Visibility::Public {
            println!(
                "  Status:       public but not yet exported — run `kask skill publish {}`",
                name
            );
        } else {
            println!("  Status:       private (not exported)");
        }
    }
}

/// Publish a skill from the private zone to the public zone.
fn skill_publish(name: &str) {
    let root = project_root();

    match skill::publish_skill(&root, name) {
        Ok(result) => {
            println!(
                "Published '{}' as '{}' to public zone: {}",
                result.name,
                result.namespaced_name,
                result.public_dir.display()
            );
            println!("  Sortable by replicant: {}", result.namespace);
            println!("  Sortable by skill:    {}", result.name);
        }
        Err(e) => {
            eprintln!("Publish failed: {e}");
            std::process::exit(1);
        }
    }
}

/// Run the dual-layer skill audit and emit a report.
///
/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  fail_below is in [0.0, 1.0]
/// post: JSON or table report printed; process exits 1 if any score < fail_below
fn skill_audit(fail_below: f64, json: bool) {
    let root = project_root();
    let mut registry = Registry::new();
    let loader = SkillLoader::new(&root);
    let _load_result = loader.load_into(&mut registry);

    let auditor = SkillAuditor::new(&registry, &registry, &root);

    let report = match auditor.audit_all() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Audit failed: {e}");
            std::process::exit(1);
        }
    };

    let threshold = fail_below.clamp(0.0, 1.0);
    let mut failed = false;

    if json {
        match report.to_json() {
            Ok(output) => println!("{output}"),
            Err(e) => {
                eprintln!("Failed to serialize audit report: {e}");
                std::process::exit(1);
            }
        }
    } else {
        let skill_entries: Vec<_> = report
            .entries
            .iter()
            .filter(|e| e.category == "skill")
            .collect();
        let non_skill_entries: Vec<_> = report
            .entries
            .iter()
            .filter(|e| e.category != "skill")
            .collect();

        println!("Skill audit report (fail_below={threshold:.2})");
        println!();
        println!(
            "{:<30} {:>6} {:>14} {:>8} defects",
            "skill", "score", "status", "active"
        );
        for entry in &skill_entries {
            let active = if entry.is_active() { "yes" } else { "no" };
            let status_label = status_label(entry.status);
            println!(
                "{:<30} {:>6.2} {:>14} {:>8} {}",
                entry.skill_name,
                entry.health_score,
                status_label,
                active,
                entry.defects.len()
            );
            if !entry.defects.is_empty() {
                for defect in &entry.defects {
                    println!("    - {defect}");
                }
            }
            if entry.health_score < threshold {
                failed = true;
            }
        }
        println!();
        println!(
            "Skills active: {}/{}",
            report.active_count(),
            skill_entries.len(),
        );
        if !non_skill_entries.is_empty() {
            println!(
                "Non-skill template crates audited (infrastructure, not counted as skills): {}",
                non_skill_entries.len()
            );
            for entry in &non_skill_entries {
                println!(
                    "  {:<30} category={}  score={:.2}  defects={}",
                    entry.skill_name,
                    entry.category,
                    entry.health_score,
                    entry.defects.len()
                );
            }
        }
    }

    if failed {
        eprintln!("Audit failed: one or more skills are below threshold.");
        std::process::exit(1);
    }
}

fn status_label(status: SkillStatus) -> &'static str {
    match status {
        SkillStatus::Active => "active",
        SkillStatus::StaleWarning => "stale",
        SkillStatus::Critical => "critical",
        SkillStatus::RecommendDeprecation => "deprecate",
    }
}

/// Resolve a default inference port for a standalone CLI command.
/// Mirrors `commands::bundle::resolve_composition_port`.
fn resolve_default_inference_port() -> std::sync::Arc<dyn InferencePort> {
    let inference_config = hkask_inference::InferenceConfig::from_env();
    let default_model = inference_config.default_model.clone();
    let ctx =
        hkask_services_core::InferenceContext::from_parts(None, &default_model, inference_config);
    commands::helpers::or_exit(
        hkask_services_core::InferenceService::resolve_port(&ctx, &default_model),
        "Failed to initialize inference port for SKILL.md derivation",
    )
}

/// Derive (reverse-translate) the SKILL.md companion from a registry crate.
///
/// Builds the **structural skeleton mechanically** from the registry crate
/// (frontmatter, registry templates table, constraints, warnings) — these are
/// copies of registry fields and need no LLM. Uses the default inference model
/// **only** to synthesize the two prose sections ("When to Use", "Instructions")
/// via `skill-maintenance/skill-maintenance-prose`, emitted as raw markdown
/// (not JSON-wrapped). Assembles skeleton + prose and writes
/// `.agents/skills/<name>/SKILL.md`.
///
/// This is the P5.1 derivation path: SKILL.md is generated from the registry,
/// not hand-authored. The structural parts come straight from the registry
/// (always in sync); only the prose is LLM-synthesized. Warnings (lexicon
/// drift, missing template files) are computed mechanically.
///
/// expect: "The system derives skill companions from the canonical registry"
/// pre:  `name` matches a `registry/templates/<name>/` crate with a `category: skill` FlowDef manifest; inference is configured
/// post: writes `.agents/skills/<name>/SKILL.md` and prints any derivation warnings
fn skill_derive(name: &str) {
    let root = project_root();
    let crate_dir = root.join("registry").join("templates").join(name);
    if !crate_dir.exists() {
        eprintln!(
            "Skill '{}' not found in registry (no {}).",
            name,
            crate_dir.display()
        );
        std::process::exit(1);
    }

    // Guard: SKILL.md is only for confirmed agent skills. The authoritative
    // skill discriminator is the `.agents/skills/<name>/` directory — the
    // curated set of agent-facing skills. If that directory doesn't exist,
    // the crate is infrastructure (MCP-server templates, CNS spans, chat
    // personas, platform auditors, pipelines, daemon-processes) and must not
    // get a derived SKILL.md. This prevents the conflation of skills with
    // infrastructure that shares the template-crate / FlowDef form.
    let skill_dir = root.join(".agents").join("skills").join(name);
    if !skill_dir.is_dir() {
        eprintln!(
            "'{name}' is not a confirmed skill (no {dir} directory). SKILL.md is only for skills in the curated .agents/skills/ set.",
            dir = skill_dir.display()
        );
        std::process::exit(1);
    }

    // ── Parse the registry crate manifest.yaml (mechanical skeleton source) ──
    let manifest_yaml =
        std::fs::read_to_string(crate_dir.join("manifest.yaml")).unwrap_or_else(|e| {
            eprintln!(
                "Failed to read {}: {e}",
                crate_dir.join("manifest.yaml").display()
            );
            std::process::exit(1);
        });
    let crate_manifest: CrateManifest =
        serde_yaml_neo::from_str(&manifest_yaml).unwrap_or_else(|e| {
            eprintln!(
                "Failed to parse {}: {e}",
                crate_dir.join("manifest.yaml").display()
            );
            std::process::exit(1);
        });
    let crate_name = crate_manifest
        .crate_
        .name
        .clone()
        .unwrap_or_else(|| name.to_string());
    let crate_desc = crate_manifest
        .crate_
        .description
        .clone()
        .unwrap_or_default();

    // ── Read each .j2 template: body (for the LLM) + light frontmatter parse ──
    let mut template_contents = String::new();
    let mut j2_frontmatters: std::collections::HashMap<String, J2Lite> =
        std::collections::HashMap::new();
    let mut warnings: Vec<String> = Vec::new();

    for entry in &crate_manifest.templates {
        let path = entry.path.clone().unwrap_or_default();
        let j2_path = crate_dir.join(&path);
        let body = std::fs::read_to_string(&j2_path).unwrap_or_else(|_| {
            warnings.push(format!(
                "{path}: referenced in manifest but file not found at {}",
                j2_path.display()
            ));
            String::new()
        });
        if !body.is_empty() {
            template_contents.push_str(&format!("\n\n--- {path} ---\n{body}"));
            let lite = parse_j2_lite(&body);
            // Lexicon drift warning (mechanical): manifest lexicon vs .j2 lexicon.
            if let (Some(man_lex), Some(j2_lex)) =
                (entry.lexicon_terms.as_ref(), lite.lexicon_terms.as_ref())
            {
                let shared: Vec<&str> = man_lex
                    .iter()
                    .filter(|t| j2_lex.contains(t))
                    .map(|s| s.as_str())
                    .collect();
                let only_j2: Vec<&str> = j2_lex
                    .iter()
                    .filter(|t| !man_lex.contains(t))
                    .map(|s| s.as_str())
                    .collect();
                if shared.len() < man_lex.len() || !only_j2.is_empty() {
                    warnings.push(format!(
                        "{path}: lexicon drift — manifest={:?} vs .j2={:?} (shared={:?}, only-in-.j2={:?})",
                        man_lex, j2_lex, shared, only_j2
                    ));
                }
            }
            j2_frontmatters.insert(path, lite);
        }
    }

    // ── LLM prose call: synthesize "When to Use" + "Instructions" (raw markdown) ──
    let inference = resolve_default_inference_port();
    let params = LLMParameters {
        max_tokens: 4096,
        temperature: 0.2,
        ..LLMParameters::default()
    };
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| {
        eprintln!("Failed to create runtime: {e}");
        std::process::exit(1);
    });

    eprintln!("Deriving SKILL.md for '{name}' (mechanical skeleton + LLM prose)…");
    let prose_prompt = render_minijinja(
        &std::fs::read_to_string(
            root.join("registry")
                .join("templates")
                .join("skill-maintenance")
                .join("skill-maintenance-prose.j2"),
        )
        .unwrap_or_else(|e| {
            eprintln!("Failed to read skill-maintenance-prose.j2: {e}");
            std::process::exit(1);
        }),
        name,
        &manifest_yaml,
        &template_contents,
    );
    let prose_result = rt
        .block_on(inference.generate(&prose_prompt, &params, None))
        .unwrap_or_else(|e| {
            eprintln!("Prose derivation failed: {e}");
            std::process::exit(1);
        });
    let prose = prose_result.text.trim();

    // ── Assemble the SKILL.md: frontmatter + title + prose + registry table + constraints ──
    let mut skill_md = String::new();
    skill_md.push_str("---\n");
    skill_md.push_str(&format!("name: {crate_name}\n"));
    skill_md.push_str("visibility: public\n");
    skill_md.push_str(&format!(
        "description: \"{}\"\n",
        crate_desc.replace('"', "\\\"")
    ));
    skill_md.push_str("---\n\n");
    skill_md.push_str(&format!("# {}\n\n", title_case(name)));
    skill_md.push_str(&format!("{crate_desc}\n\n"));
    skill_md.push_str(prose);
    if !prose.ends_with('\n') {
        skill_md.push('\n');
    }
    skill_md.push('\n');

    // Registry Templates table (mechanical, from manifest entries).
    skill_md.push_str("## Registry Templates\n\n");
    skill_md.push_str("| Template | Type | Purpose |\n");
    skill_md.push_str("|----------|------|---------|\n");
    for entry in &crate_manifest.templates {
        let path = entry.path.as_deref().unwrap_or("?");
        let ttype = entry.template_type.as_deref().unwrap_or("?");
        let desc = entry
            .description
            .as_deref()
            .unwrap_or("")
            .replace('\n', " ");
        skill_md.push_str(&format!("| `{path}` | {ttype} | {desc} |\n"));
    }
    skill_md.push('\n');

    // Constraints (mechanical, from .j2 frontmatter + standard safety rules).
    skill_md.push_str("## Constraints\n\n");
    for entry in &crate_manifest.templates {
        let path = entry.path.as_deref().unwrap_or("?");
        if let Some(lite) = j2_frontmatters.get(path) {
            let vis = lite.visibility.as_deref().unwrap_or("Public");
            if let Some(ec) = lite.energy_cap {
                skill_md.push_str(&format!("- `{path}`: {vis}, energy_cap {ec}.\n"));
            } else {
                skill_md.push_str(&format!("- `{path}`: {vis}.\n"));
            }
        }
    }
    skill_md.push_str("- Registry is authoritative — when this SKILL.md disagrees with registry templates, the registry wins.\n");

    // ── Write the derived SKILL.md ──
    let dest_dir = root.join(".agents").join("skills").join(name);
    std::fs::create_dir_all(&dest_dir).unwrap_or_else(|e| {
        eprintln!("Failed to create {}: {e}", dest_dir.display());
        std::process::exit(1);
    });
    let dest = dest_dir.join("SKILL.md");
    std::fs::write(&dest, &skill_md).unwrap_or_else(|e| {
        eprintln!("Failed to write {}: {e}", dest.display());
        std::process::exit(1);
    });

    println!("Derived SKILL.md → {}", dest.display());
    if !warnings.is_empty() {
        eprintln!("Warnings:");
        for w in &warnings {
            eprintln!("  - {w}");
        }
    }
}

// ── Helpers for skill_derive ────────────────────────────────────────────────

/// Minimal registry-crate manifest.yaml shape (the template crate's manifest,
/// NOT the FlowDef skill manifest).
#[derive(Debug, serde::Deserialize)]
struct CrateManifest {
    #[serde(default, rename = "crate")]
    crate_: CrateHeader,
    #[serde(default)]
    templates: Vec<CrateTemplateEntry>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct CrateHeader {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct CrateTemplateEntry {
    #[serde(default)]
    path: Option<String>,
    #[serde(default, rename = "type")]
    template_type: Option<String>,
    #[serde(default)]
    lexicon_terms: Option<Vec<String>>,
    #[serde(default)]
    description: Option<String>,
}

/// Light .j2 frontmatter fields needed for constraints + drift warnings.
#[derive(Debug, Default)]
struct J2Lite {
    lexicon_terms: Option<Vec<String>>,
    energy_cap: Option<i64>,
    visibility: Option<String>,
}

/// Parse lexicon_terms, energy_cap, and visibility from a .j2's [inference] block.
fn parse_j2_lite(content: &str) -> J2Lite {
    let mut in_block = false;
    let mut lex: Vec<String> = Vec::new();
    let mut energy: Option<i64> = None;
    let mut vis: Option<String> = None;
    let mut in_lex = false;
    for line in content.lines() {
        if line.trim_start().starts_with("[inference]") {
            in_block = true;
            in_lex = false;
            continue;
        }
        if in_block {
            if (line.starts_with('"')
                || (!line.starts_with(' ') && !line.starts_with('\t') && !line.starts_with('-')))
                && !line.is_empty()
                && !line.starts_with(' ')
                && !line.starts_with('\t')
            {
                in_block = false;
                in_lex = false;
                continue;
            }
            let trimmed = line.trim_start();
            if trimmed.starts_with("lexicon_terms:") {
                in_lex = true;
                let rest = trimmed.strip_prefix("lexicon_terms:").unwrap_or("").trim();
                if let Some(arr) = parse_lex_inline(rest) {
                    lex.extend(arr);
                    in_lex = false;
                }
                continue;
            }
            if in_lex {
                if let Some(item) = trimmed.strip_prefix('-') {
                    lex.push(item.trim().to_string());
                    continue;
                }
                in_lex = false;
            }
            if let Some(rest) = trimmed.strip_prefix("energy_cap:") {
                energy = rest.trim().parse().ok();
            }
            if let Some(rest) = trimmed.strip_prefix("visibility:") {
                vis = Some(rest.trim().to_string());
            }
        }
    }
    J2Lite {
        lexicon_terms: if lex.is_empty() { None } else { Some(lex) },
        energy_cap: energy,
        visibility: vis,
    }
}

/// Parse an inline lexicon list like `[a, b, c]`.
fn parse_lex_inline(s: &str) -> Option<Vec<String>> {
    let s = s.trim();
    if !s.starts_with('[') {
        return None;
    }
    let inner = s.trim_start_matches('[').trim_end_matches(']');
    if inner.is_empty() {
        return Some(Vec::new());
    }
    Some(inner.split(',').map(|t| t.trim().to_string()).collect())
}

/// Render the prose template with minijinja (raw markdown output — no JSON).
fn render_minijinja(
    template: &str,
    skill_name: &str,
    manifest_yaml: &str,
    template_contents: &str,
) -> String {
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Lenient);
    env.add_template("tpl", template).unwrap_or_else(|e| {
        eprintln!("Invalid prose template syntax: {e}");
        std::process::exit(1);
    });
    let ctx = json!({
        "skill_name": skill_name,
        "manifest_yaml": manifest_yaml,
        "template_contents": template_contents,
    });
    env.get_template("tpl")
        .and_then(|t| t.render(minijinja::Value::from_serialize(&ctx)))
        .unwrap_or_else(|e| {
            eprintln!("Prose template render error: {e}");
            std::process::exit(1);
        })
}

/// Title-case a skill name (e.g. "grill-me" → "Grill Me").
fn title_case(name: &str) -> String {
    name.split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
