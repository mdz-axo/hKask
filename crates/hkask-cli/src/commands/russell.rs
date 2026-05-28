//! Russell import command handlers

use crate::russell_mapper::{MappedTemplate, RussellMapper, RussellMappingConfig};
use std::path::Path;

/// Import Russell assets into hKask registry
pub fn import_russell(
    source_path: &Path,
    config: &RussellMappingConfig,
    verbose: bool,
) -> Result<Vec<MappedTemplate>, String> {
    let mapper = RussellMapper::new();
    let mut assets = Vec::new();

    if source_path.is_file() {
        let extension = source_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        match extension {
            "yaml" | "yml" => match mapper.analyze_skill_manifest(source_path) {
                Ok(manifest) => {
                    let mapped = mapper.map_to_hkask(&manifest);
                    if verbose {
                        println!("Mapped Russell manifest: {} -> {}", manifest.id, mapped.id);
                    }
                    assets.push(mapped);
                }
                Err(e) => {
                    eprintln!("Failed to analyze {}: {}", source_path.display(), e);
                    if !config.dry_run {
                        return Err(format!("Migration failed: {}", e));
                    }
                }
            },
            _ => {
                return Err(format!("Unsupported file type: {}", extension));
            }
        }
    } else if source_path.is_dir() {
        for entry in std::fs::read_dir(source_path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.is_file() {
                let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

                match extension {
                    "yaml" | "yml" => match mapper.analyze_skill_manifest(&path) {
                        Ok(manifest) => {
                            let mapped = mapper.map_to_hkask(&manifest);
                            if verbose {
                                println!(
                                    "Mapped Russell manifest: {} -> {}",
                                    manifest.id, mapped.id
                                );
                            }
                            assets.push(mapped);
                        }
                        Err(e) => {
                            eprintln!("Failed to analyze {}: {}", path.display(), e);
                            if !config.dry_run {
                                return Err(format!("Migration failed: {}", e));
                            }
                        }
                    },
                    _ => {}
                }
            } else if path.is_dir() {
                for sub_entry in std::fs::read_dir(&path).map_err(|e| e.to_string())? {
                    let sub_entry = sub_entry.map_err(|e| e.to_string())?;
                    let sub_path = sub_entry.path();

                    if sub_path.is_file() {
                        let extension = sub_path.extension().and_then(|s| s.to_str()).unwrap_or("");

                        if (extension == "yaml" || extension == "yml")
                            && sub_path.file_name().and_then(|s| s.to_str())
                                == Some("manifest.yaml")
                        {
                            match mapper.analyze_skill_manifest(&sub_path) {
                                Ok(manifest) => {
                                    let mapped = mapper.map_to_hkask(&manifest);
                                    if verbose {
                                        println!(
                                            "Mapped Russell manifest: {} -> {}",
                                            manifest.id, mapped.id
                                        );
                                    }
                                    assets.push(mapped);
                                }
                                Err(e) => {
                                    eprintln!("Failed to analyze {}: {}", sub_path.display(), e);
                                    if !config.dry_run {
                                        return Err(format!("Migration failed: {}", e));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        return Err(format!(
            "Source path does not exist: {}",
            source_path.display()
        ));
    }

    if config.dry_run {
        println!("\nDry run complete - no assets written to registry");
    }

    Ok(assets)
}

/// Import Russell assets into hKask registry using an existing mapper
pub fn import_russell_with_mapper(
    mapper: &RussellMapper,
    source_path: &Path,
    verbose: bool,
) -> Result<Vec<MappedTemplate>, String> {
    let mut assets = Vec::new();

    if source_path.is_file() {
        let extension = source_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        match extension {
            "yaml" | "yml" => match mapper.analyze_skill_manifest(source_path) {
                Ok(manifest) => {
                    let mapped = mapper.map_to_hkask(&manifest);
                    if verbose {
                        println!("Mapped Russell manifest: {} -> {}", manifest.id, mapped.id);
                    }
                    assets.push(mapped);
                }
                Err(e) => {
                    eprintln!("Failed to analyze {}: {}", source_path.display(), e);
                    return Err(format!("Migration failed: {}", e));
                }
            },
            _ => {
                return Err(format!("Unsupported file type: {}", extension));
            }
        }
    } else if source_path.is_dir() {
        for entry in std::fs::read_dir(source_path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            if path.is_file() {
                let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

                match extension {
                    "yaml" | "yml" => match mapper.analyze_skill_manifest(&path) {
                        Ok(manifest) => {
                            let mapped = mapper.map_to_hkask(&manifest);
                            if verbose {
                                println!(
                                    "Mapped Russell manifest: {} -> {}",
                                    manifest.id, mapped.id
                                );
                            }
                            assets.push(mapped);
                        }
                        Err(e) => {
                            eprintln!("Failed to analyze {}: {}", path.display(), e);
                        }
                    },
                    _ => {}
                }
            } else if path.is_dir() {
                for sub_entry in std::fs::read_dir(&path).map_err(|e| e.to_string())? {
                    let sub_entry = sub_entry.map_err(|e| e.to_string())?;
                    let sub_path = sub_entry.path();

                    if sub_path.is_file() {
                        let extension = sub_path.extension().and_then(|s| s.to_str()).unwrap_or("");

                        if (extension == "yaml" || extension == "yml")
                            && sub_path.file_name().and_then(|s| s.to_str())
                                == Some("manifest.yaml")
                        {
                            match mapper.analyze_skill_manifest(&sub_path) {
                                Ok(manifest) => {
                                    let mapped = mapper.map_to_hkask(&manifest);
                                    if verbose {
                                        println!(
                                            "Mapped Russell manifest: {} -> {}",
                                            manifest.id, mapped.id
                                        );
                                    }
                                    assets.push(mapped);
                                }
                                Err(e) => {
                                    eprintln!("Failed to analyze {}: {}", sub_path.display(), e);
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        return Err(format!(
            "Source path does not exist: {}",
            source_path.display()
        ));
    }

    Ok(assets)
}
