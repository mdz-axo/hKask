//! Skill service — visibility management and publishing.
//! # REQ: P11 (Digital Public/Private Sphere) — private→public skill export with namespace.
//! # expect: "The service layer exposes minimal, essential interfaces shared by all surfaces" [P5]
//!
//! Implements the two-zone skill model: `.agents/skills/` (private source)
//! → `skills/` (public export surface). Handles replicant name resolution,
//! BLAKE3 content hashing, SKILL.md front matter parsing and mutation,
//! zone-aware discovery, and publishing.
//!
//! ℏKask - A Minimal Viable Container for Agents

use hkask_rsolidity::contract;

use hkask_templates::SkillLoader;
use hkask_types::ports::{Skill, SkillZone};
use hkask_types::visibility::Visibility;

use std::fs;
use std::path::{Path, PathBuf};

use hkask_services_core::ServiceError;

/// Result of publishing a skill from private to public zone.
#[derive(Debug)]
pub struct SkillPublishResult {
    /// Original skill name.
    pub name: String,
    /// Namespaced name in the public zone (`<namespace>--<name>`).
    pub namespaced_name: String,
    /// Replicant namespace used for publishing.
    pub namespace: String,
    /// Path to the published skill directory.
    pub public_dir: PathBuf,
}

/// Discovered skill metadata.
#[derive(Debug)]
pub struct SkillInfo {
    /// Skill directory path.
    pub path: PathBuf,
    /// Skill name (directory name).
    pub name: String,
    /// Visibility parsed from SKILL.md.
    pub visibility: Visibility,
    /// Namespace parsed from SKILL.md, if present.
    pub namespace: Option<String>,
    /// BLAKE3 content hash of SKILL.md, if computable.
    pub content_hash: Option<String>,
}

/// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  zone_dir must be a readable directory; each subdirectory with SKILL.md is treated as a skill
/// post: returns Vec<SkillInfo> sorted by name, each with path, name, visibility, namespace, and content_hash; Err on I/O failure
#[contract(id = "P5-svc-skill-088", principle = "P5")]
pub fn discover_skills(zone_dir: &Path) -> Result<Vec<SkillInfo>, ServiceError> {
    let mut skills = Vec::new();
    let entries = fs::read_dir(zone_dir).map_err(|e| {
        let msg = format!("Error scanning {}: {e}", zone_dir.display());
        ServiceError::Skill {
            source: Some(Box::new(e)),
            message: msg,
        }
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            let msg = format!("Error reading directory: {e}");
            ServiceError::Skill {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;
        let path = entry.path();
        if path.is_dir() && path.join("SKILL.md").exists() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();
            let skill_md = path.join("SKILL.md");
            let visibility = read_skill_visibility(&skill_md);
            let namespace = read_skill_namespace(&skill_md);
            let content_hash = compute_content_hash(&skill_md);
            skills.push(SkillInfo {
                path,
                name,
                visibility,
                namespace,
                content_hash,
            });
        }
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    // REQ: P9-CNS-SVC-020 pre: valid zone_dir, post: cns.skill span emitted
    // expect: "The service layer provides CNS health and regulation queries" [P9]
    // P9: CNS span
    tracing::info!(
        target: "cns.skill",
        operation = "skills_discovered",
        zone_dir = %zone_dir.display(),
        count = skills.len(),
        "CNS"
    );
    Ok(skills)
}

/// Read the visibility field from a SKILL.md file.
///
/// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  skill_md_path may or may not exist; if unreadable, defaults to Private
/// post: returns Visibility parsed from front matter; defaults to Private on any parse failure
#[contract(id = "P5-svc-skill-089", principle = "P5")]
pub fn read_skill_visibility(skill_md_path: &Path) -> Visibility {
    let content = match fs::read_to_string(skill_md_path) {
        Ok(c) => c,
        Err(_) => return Visibility::Private,
    };

    let fm = SkillLoader::parse_front_matter(&content);
    match fm {
        Ok(front_matter) => front_matter
            .visibility
            .as_deref()
            .and_then(Visibility::parse_str)
            .unwrap_or(Visibility::Private),
        Err(_) => Visibility::Private,
    }
}

/// Compute BLAKE3 hash of a SKILL.md file's contents.
fn compute_content_hash(skill_md_path: &Path) -> Option<String> {
    let content = fs::read_to_string(skill_md_path).ok()?;
    let hash = hkask_types::text::blake3_hash(content.as_bytes());
    Some(hex::encode(hash))
}

/// Read the namespace field from a SKILL.md file.
///
/// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  skill_md_path may or may not exist; returns None if unreadable or no namespace in front matter
/// post: returns Some(namespace) if front matter has a namespace field; None otherwise
#[contract(id = "P5-svc-skill-090", principle = "P5")]
pub fn read_skill_namespace(skill_md_path: &Path) -> Option<String> {
    let content = fs::read_to_string(skill_md_path).ok()?;
    let fm = SkillLoader::parse_front_matter(&content).ok()?;
    fm.namespace
}

/// Compute BLAKE3 hash of an arbitrary file's contents.
///
/// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  path must be a readable file; returns None if unreadable
/// post: returns Some(hex-encoded BLAKE3 hash) on success; None on I/O failure
#[contract(id = "P5-svc-skill-091", principle = "P5")]
pub fn compute_file_hash(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let hash = hkask_types::text::blake3_hash(content.as_bytes());
    Some(hex::encode(hash))
}

/// Find a skill in the public zone by its base name.
///
/// Searches for any `<namespace>--<name>` directory that ends with `--<name>`.
///
/// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  root must be a valid skill zone root; name must be non-empty
/// post: returns Some(PathBuf) to the matching skill directory if found; None if no match or public zone missing
#[contract(id = "P5-svc-skill-092", principle = "P5")]
pub fn find_public_skill(root: &Path, name: &str) -> Option<PathBuf> {
    let public_dir = root.join(SkillZone::Public.directory());
    if !public_dir.exists() {
        return None;
    }

    let suffix = format!("--{}", name);
    let entries = fs::read_dir(&public_dir).ok()?;
    for entry in entries {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_dir()
            && path.join("SKILL.md").exists()
            && let Some(dir_name) = path.file_name().and_then(|n| n.to_str())
            && dir_name.ends_with(&suffix)
            && Skill::parse_qualified_id(dir_name).is_some()
        {
            return Some(path);
        }
    }
    None
}

/// Publish a skill from the private zone to the public zone.
///
/// Copies the skill directory, updates visibility and namespace in the
/// exported copy's SKILL.md. The public copy is a snapshot, not a live link.
///
/// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  root must be a valid skill zone root; name must exist in the private zone
/// post: skill directory is copied to public zone with namespaced name; visibility set to public; namespace set to replicant name; Err if private skill not found
#[contract(id = "P5-svc-skill-093", principle = "P5")]
pub fn publish_skill(root: &Path, name: &str) -> Result<SkillPublishResult, ServiceError> {
    let private_dir = root.join(SkillZone::Private.directory()).join(name);

    if !private_dir.exists() {
        return Err(ServiceError::Skill {
            source: None,
            message: format!("Skill '{name}' not found in private zone"),
        });
    }

    let replicant_name = resolve_replicant_name();
    let namespaced_name = format!("{}--{}", replicant_name, name);
    let public_dir = root
        .join(SkillZone::Public.directory())
        .join(&namespaced_name);

    // Ensure public zone exists
    let public_zone = root.join(SkillZone::Public.directory());
    if !public_zone.exists() {
        fs::create_dir_all(&public_zone).map_err(|e| {
            let msg = format!(
                "Failed to create public zone {}: {e}",
                public_zone.display()
            );
            ServiceError::Skill {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;
    }

    // Remove existing public copy before replacing
    if public_dir.exists() {
        fs::remove_dir_all(&public_dir).map_err(|e| {
            let msg = format!(
                "Failed to remove existing public copy {}: {e}",
                public_dir.display()
            );
            ServiceError::Skill {
                source: Some(Box::new(e)),
                message: msg,
            }
        })?;
    }

    // Copy the skill directory
    copy_dir_recursive(&private_dir, &public_dir).map_err(|e| {
        let msg = format!("Failed to copy skill to public zone: {e}");
        ServiceError::Skill {
            source: None,
            message: msg,
        }
    })?;

    // Update the SKILL.md visibility and namespace in the exported copy
    let public_skill_md = public_dir.join("SKILL.md");
    update_visibility_in_skill_md(&public_skill_md, "public");
    update_namespace_in_skill_md(&public_skill_md, &replicant_name);

    // REQ: P9-CNS-SVC-021 pre: valid skill publish, post: cns.skill span emitted
    // expect: "The service layer provides CNS health and regulation queries" [P9]
    // P9: CNS span
    tracing::info!(
        target: "cns.skill",
        operation = "skill_published",
        name = %name,
        namespaced_name = %namespaced_name,
        namespace = %replicant_name,
        "CNS"
    );

    Ok(SkillPublishResult {
        name: name.to_string(),
        namespaced_name,
        namespace: replicant_name,
        public_dir,
    })
}

/// Resolve the replicant name for skill namespacing.
///
/// Resolution order:
/// 1. `HKASK_REPLICANT_NAME` env var (explicit override)
/// 2. Git config `user.name` (if in a git repo)
/// 3. Fallback: "local"
///
/// [P5] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  none (always succeeds)
/// post: returns a non-empty String — env var, git user.name, or "local" fallback
#[contract(id = "P5-svc-skill-094", principle = "P5")]
pub fn resolve_replicant_name() -> String {
    if let Ok(name) = std::env::var("HKASK_REPLICANT_NAME")
        && !name.is_empty()
    {
        return name;
    }

    if let Ok(output) = std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        && output.status.success()
    {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return name;
        }
    }

    "local".to_string()
}

// ── Internal helpers ────────────────────────────────────────────────────

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;

    let entries = fs::read_dir(src).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(src_path.file_name().unwrap_or_default());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

/// Update the `visibility` field in a SKILL.md YAML front matter.
fn update_visibility_in_skill_md(path: &Path, visibility: &str) {
    if let Ok(content) = fs::read_to_string(path) {
        let updated = if content.contains("visibility:") {
            content
                .lines()
                .map(|line| {
                    if line.trim().starts_with("visibility:") {
                        let indent = line.len() - line.trim_start().len();
                        format!("{}visibility: {}", " ".repeat(indent), visibility)
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else if content.contains("name:") {
            content
                .lines()
                .flat_map(|line| {
                    let mut result = vec![line.to_string()];
                    if line.trim().starts_with("name:") {
                        let indent = line.len() - line.trim_start().len();
                        result.push(format!("{}visibility: {}", " ".repeat(indent), visibility));
                    }
                    result
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            content
        };

        let _ = fs::write(path, updated);
    }
}

/// Update or add the `namespace` field in a SKILL.md YAML front matter.
fn update_namespace_in_skill_md(path: &Path, namespace: &str) {
    if let Ok(content) = fs::read_to_string(path) {
        let updated = if content.contains("namespace:") {
            content
                .lines()
                .map(|line| {
                    if line.trim().starts_with("namespace:") {
                        let indent = line.len() - line.trim_start().len();
                        format!("{}namespace: {}", " ".repeat(indent), namespace)
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else if content.contains("visibility:") {
            content
                .lines()
                .flat_map(|line| {
                    let mut result = vec![line.to_string()];
                    if line.trim().starts_with("visibility:") {
                        let indent = line.len() - line.trim_start().len();
                        result.push(format!("{}namespace: {}", " ".repeat(indent), namespace));
                    }
                    result
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else if content.contains("name:") {
            content
                .lines()
                .flat_map(|line| {
                    let mut result = vec![line.to_string()];
                    if line.trim().starts_with("name:") {
                        let indent = line.len() - line.trim_start().len();
                        result.push(format!("{}namespace: {}", " ".repeat(indent), namespace));
                    }
                    result
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            content
        };

        let _ = fs::write(path, updated);
    }
}

// ── Tests ───────────────────────────────────────────────────────────────
