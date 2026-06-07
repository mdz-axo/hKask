//! Skill command handlers for `kask skill`
//
//! Implements CLI display logic for skill visibility management.
//! Two-zone model: `.agents/skills/` (source) → `skills/` (export surface).

use crate::cli::SkillAction;
use hkask_types::ports::{Skill, SkillZone};
use hkask_types::visibility::Visibility;
use std::fs;
use std::path::{Path, PathBuf};

/// Default project root (current directory).
fn project_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Resolve the replicant name for skill namespacing.
///
/// The namespace is always a user replicant handle — system agents don't author skills.
/// Resolution order:
/// 1. `HKASK_REPLICANT_NAME` env var (explicit override)
/// 2. Git config `user.name` (if in a git repo)
/// 3. Fallback: "local" (indicates skills not yet published to a shared remote)
fn resolve_replicant_name() -> String {
    if let Ok(name) = std::env::var("HKASK_REPLICANT_NAME") {
        if !name.is_empty() {
            return name;
        }
    }

    // Try git user.name as a reasonable default
    if let Ok(output) = std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
    {
        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                return name;
            }
        }
    }

    // Fallback — indicates skills are from a local-only instance
    "local".to_string()
}

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

        let skill_dirs = match discover_skills(&zone_dir) {
            Ok(dirs) => dirs,
            Err(e) => {
                eprintln!("Error scanning {}: {}", zone_dir.display(), e);
                continue;
            }
        };

        if skill_dirs.is_empty() {
            continue;
        }

        println!("  {} zone ({}):", zone.as_str(), zone_dir.display());

        for skill_dir in skill_dirs {
            let name = skill_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?");

            // Parse visibility from SKILL.md
            let vis = read_skill_visibility(&skill_dir.join("SKILL.md"));

            // Apply filter
            if let Some(filter) = vis_filter {
                if vis != filter {
                    continue;
                }
            }

            let vis_str = vis.as_str();
            let ns = read_skill_namespace(&skill_dir.join("SKILL.md"));
            let hash = read_skill_content_hash(&skill_dir.join("SKILL.md"));
            let hash_display = hash
                .map(|h| {
                    let short = &h[..h.len().min(12)];
                    short.to_string()
                })
                .unwrap_or_else(|| "-".to_string());

            let ns_display = ns.as_deref().unwrap_or("-");
            println!(
                "    {:30} visibility={:8} namespace={:12} hash={}",
                name, vis_str, ns_display, hash_display
            );
        }
    }
}

/// Show skill status — compare private source vs published copy.
fn skill_status(name: &str) {
    let root = project_root();
    let private_dir = root.join(SkillZone::Private.directory()).join(name);

    // Find the published copy — search for any `<namespace>--<name>` directory
    // in the public zone.
    let public_dir = find_public_skill(&root, name);

    if !private_dir.exists() {
        eprintln!("Skill '{}' not found in private zone.", name);
        return;
    }

    let private_vis = read_skill_visibility(&private_dir.join("SKILL.md"));
    let private_hash = compute_file_hash(&private_dir.join("SKILL.md"));

    println!("Skill: {}", name);
    println!("  Private zone: {}", private_dir.display());
    println!("  Visibility:   {}", private_vis.as_str());
    if let Some(ref ns) = read_skill_namespace(&private_dir.join("SKILL.md")) {
        println!("  Namespace:    {}", ns);
    }
    println!(
        "  Source hash:  {}",
        private_hash.as_deref().unwrap_or("(error)")
    );

    if let Some(ref pub_dir) = public_dir {
        let public_hash = compute_file_hash(&pub_dir.join("SKILL.md"));
        let pub_namespace = read_skill_namespace(&pub_dir.join("SKILL.md"));
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
///
/// This is a one-way copy (src→dist). The public copy is a snapshot,
/// not a live link. After publishing, the two copies are independent.
///
/// The public zone uses namespaced directory names: `<namespace>--<name>/`
/// where namespace is the replicant handle (always a user replicant, never a system agent).
fn skill_publish(name: &str) {
    let root = project_root();
    let private_dir = root.join(SkillZone::Private.directory()).join(name);

    // Resolve the replicant name for namespacing.
    // The namespace is always a user replicant handle — system agents don't author skills.
    let replicant_name = resolve_replicant_name();
    let namespaced_name = format!("{}--{}", replicant_name, name);
    let public_dir = root
        .join(SkillZone::Public.directory())
        .join(&namespaced_name);

    if !private_dir.exists() {
        eprintln!("Skill '{}' not found in private zone.", name);
        std::process::exit(1);
    }

    // Ensure public zone exists
    let public_zone = root.join(SkillZone::Public.directory());
    if !public_zone.exists() {
        fs::create_dir_all(&public_zone).unwrap_or_else(|e| {
            eprintln!(
                "Failed to create public zone {}: {}",
                public_zone.display(),
                e
            );
            std::process::exit(1);
        });
    }

    // Copy the skill directory
    if public_dir.exists() {
        // Remove existing public copy before replacing
        fs::remove_dir_all(&public_dir).unwrap_or_else(|e| {
            eprintln!(
                "Failed to remove existing public copy {}: {}",
                public_dir.display(),
                e
            );
            std::process::exit(1);
        });
    }

    copy_dir_recursive(&private_dir, &public_dir).unwrap_or_else(|e| {
        eprintln!("Failed to copy skill to public zone: {}", e);
        std::process::exit(1);
    });

    // Update the SKILL.md visibility and namespace in the exported copy
    let public_skill_md = public_dir.join("SKILL.md");
    update_visibility_in_skill_md(&public_skill_md, "public");
    update_namespace_in_skill_md(&public_skill_md, &replicant_name);

    println!(
        "Published '{}' as '{}' to public zone: {}",
        name,
        namespaced_name,
        public_dir.display()
    );
    println!("  Sortable by replicant: {}", replicant_name);
    println!("  Sortable by skill:    {}", name);
}

/// Discover skill directories within a zone directory.
fn discover_skills(zone_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut skill_dirs = Vec::new();
    let entries = fs::read_dir(zone_dir).map_err(|e| e.to_string())?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() && path.join("SKILL.md").exists() {
            skill_dirs.push(path);
        }
    }

    skill_dirs.sort();
    Ok(skill_dirs)
}

/// Read the visibility field from a SKILL.md file.
fn read_skill_visibility(skill_md_path: &Path) -> Visibility {
    let content = match fs::read_to_string(skill_md_path) {
        Ok(c) => c,
        Err(_) => return Visibility::Private,
    };

    let fm = hkask_templates::SkillLoader::parse_front_matter(&content);
    match fm {
        Ok(front_matter) => front_matter
            .visibility
            .as_deref()
            .and_then(Visibility::parse_str)
            .unwrap_or(Visibility::Private),
        Err(_) => Visibility::Private,
    }
}

/// Read the content hash from a SKILL.md file (computed on-the-fly).
fn read_skill_content_hash(skill_md_path: &Path) -> Option<String> {
    compute_file_hash(skill_md_path)
}

/// Compute BLAKE3 hash of a file's contents.
fn compute_file_hash(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let hash = hkask_types::blake3_hash(content.as_bytes());
    Some(hex::encode(hash))
}

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
/// If the field exists, replace it. If not, add it after `name:`.
fn update_visibility_in_skill_md(path: &Path, visibility: &str) {
    if let Ok(content) = fs::read_to_string(path) {
        let updated = if content.contains("visibility:") {
            // Replace existing visibility line
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
            // Insert visibility after the name line
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
/// The namespace is always a user replicant handle — system agents don't author skills.
fn update_namespace_in_skill_md(path: &Path, namespace: &str) {
    if let Ok(content) = fs::read_to_string(path) {
        let updated = if content.contains("namespace:") {
            // Replace existing namespace line
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
            // Insert after the visibility line
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
            // Insert after the name line (no visibility yet)
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

/// Find a skill in the public zone by its base name.
///
/// Searches for any `<namespace>--<name>` directory that ends with `--<name>`.
/// Returns the first match.
fn find_public_skill(root: &Path, name: &str) -> Option<PathBuf> {
    let public_dir = root.join(SkillZone::Public.directory());
    if !public_dir.exists() {
        return None;
    }

    let suffix = format!("--{}", name);
    let entries = fs::read_dir(&public_dir).ok()?;
    for entry in entries {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_dir() && path.join("SKILL.md").exists() {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if dir_name.ends_with(&suffix) {
                    // Verify it's a valid qualified ID
                    if Skill::parse_qualified_id(dir_name).is_some() {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

/// Read the namespace field from a SKILL.md file.
fn read_skill_namespace(skill_md_path: &Path) -> Option<String> {
    let content = fs::read_to_string(skill_md_path).ok()?;
    let fm = hkask_templates::SkillLoader::parse_front_matter(&content).ok()?;
    fm.namespace
}
