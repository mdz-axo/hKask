//! Keystore command handlers for `kask keystore`
//!
//! Implements the CLI display logic for key management operations.

use crate::cli::KeystoreAction;
use rand::RngCore;
use std::io::Write;

pub fn run(action: KeystoreAction) {
    let keychain = hkask_keystore::Keychain::default();

    match action {
        KeystoreAction::Load {
            path,
            prefix,
            overwrite,
            shred,
        } => {
            // ── Phase 1: Parse and validate (no state changes yet) ──
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to read {}: {}", path.display(), e);
                    std::process::exit(1);
                }
            };

            let mut entries: Vec<(String, String)> = Vec::new();
            let mut skipped_existing = 0usize;

            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim().to_string();
                    let value = value.trim().to_string();
                    if !key.starts_with(&prefix) {
                        continue;
                    }
                    if value.is_empty() {
                        continue;
                    }
                    if !overwrite && keychain.retrieve_by_key(&key).is_ok() {
                        skipped_existing += 1;
                        continue;
                    }
                    entries.push((key, value));
                }
            }

            if entries.is_empty() {
                if skipped_existing > 0 {
                    println!(
                        "All {} keys already in keychain (use --overwrite to replace).",
                        skipped_existing
                    );
                } else {
                    println!("No keys found in {}.", path.display());
                }
                return;
            }

            // ── Phase 2: Show summary, get consent if shredding ──
            println!();
            println!("  Keys to load from {}:", path.display());
            for (key, _value) in &entries {
                println!("    {}", key);
            }
            if skipped_existing > 0 {
                println!("  ({} already in keychain — skipped)", skipped_existing);
            }
            println!();

            if shred {
                // Affirmative consent gate before destruction (Magna Carta P2)
                println!("  ═══════════════════════════════════════════════════════════");
                println!("  ⚠️  FILE DESTRUCTION WARNING");
                println!("  ═══════════════════════════════════════════════════════════");
                println!();
                println!(
                    "  After loading, {} will be PERMANENTLY DELETED.",
                    path.display()
                );
                println!("  The file will be overwritten with random data, then removed.");
                println!();
                println!("  BEFORE continuing:");
                println!("  ☐ Do you have a backup of these keys elsewhere?");
                println!("    (password manager, encrypted USB, your local machine)");
                println!("  ☐ You will NOT be able to recover keys from this file");
                println!("    after this step.");
                println!();
                println!("  ═══════════════════════════════════════════════════════════");
                println!();
                print!("  Load keys and shred {}? [y/n/q]: ", path.display());
                std::io::stdout().flush().ok();

                let mut input = String::new();
                if std::io::stdin().read_line(&mut input).is_err() {
                    println!("  Aborted.");
                    return;
                }
                match input.trim().to_lowercase().as_str() {
                    "y" | "yes" => {
                        // Proceed — store then shred
                    }
                    "n" | "no" => {
                        println!();
                        println!("  Keys will be loaded into the keychain.");
                        println!(
                            "  File {} will be KEPT on disk — delete it yourself when ready.",
                            path.display()
                        );
                        println!(
                            "  (Use `shred -u {}` to securely delete it later.)",
                            path.display()
                        );
                        // Fall through to store-only
                    }
                    _ => {
                        println!("  Aborted — nothing stored, nothing deleted.");
                        return;
                    }
                }
            }

            // ── Phase 3: Store keys in keychain ──
            println!();
            let mut loaded = 0usize;
            let mut failed = 0usize;
            for (key, value) in &entries {
                match keychain.store_by_key(key, value) {
                    Ok(()) => {
                        println!("  ✓ stored {}", key);
                        loaded += 1;
                    }
                    Err(e) => {
                        eprintln!("  ✗ failed {} : {}", key, e);
                        failed += 1;
                    }
                }
            }
            println!();
            println!("  Loaded {} keys", loaded);
            if failed > 0 {
                println!("  Failed: {} keys (check keychain permissions)", failed);
            }
            if skipped_existing > 0 {
                println!("  Skipped: {} (already in keychain)", skipped_existing);
            }

            // ── Phase 4: Shred if consented ──
            if shred && failed == 0 {
                println!();
                print!("  Shredding {}... ", path.display());
                std::io::stdout().flush().ok();

                match secure_delete_file(&path) {
                    Ok(()) => println!("✓ deleted"),
                    Err(e) => {
                        eprintln!();
                        eprintln!("  ✗ Failed to shred: {}", e);
                        eprintln!(
                            "  Keys are safe in keychain. Delete {} manually when ready.",
                            path.display()
                        );
                    }
                }
            }

            if shred && failed == 0 {
                println!();
                println!("  Setup complete. Run: kask chat");
            }
        }
        KeystoreAction::List => {
            eprintln!(
                "OS keychain does not support listing. Use 'kask keystore get <KEY>' to check individual keys."
            );
        }
        KeystoreAction::Get { key } => {
            let val = super::helpers::or_exit(keychain.retrieve_by_key(&key), "Key not found");
            if val.len() > 8 {
                println!("{}={}**{}", key, &val[..4], &val[val.len() - 4..]);
            } else {
                println!("{}=****", key);
            }
        }
        KeystoreAction::Set { key, value } => {
            super::helpers::or_exit(keychain.store_by_key(&key, &value), "Failed to store key");
            println!("Stored {}", key);
        }
        KeystoreAction::Delete { key } => {
            super::helpers::or_exit(keychain.delete_by_key(&key), "Failed to delete key");
            println!("Deleted {}", key);
        }
    }
}

/// Securely delete a file by overwriting with random bytes before unlinking.
///
/// Writes random data equal to the file's current size (capped at 64 KiB for
/// large files), syncs to disk, then removes the file. Not cryptographic-grade
/// (no multi-pass), but sufficient to prevent casual recovery of API keys from
/// a cloud server's disk.
pub(crate) fn secure_delete_file(path: &std::path::Path) -> Result<(), String> {
    let metadata =
        std::fs::metadata(path).map_err(|e| format!("Cannot read file metadata: {}", e))?;
    let len = metadata.len().min(65536); // Cap at 64 KiB

    // Overwrite with random bytes
    let mut random_bytes = vec![0u8; len as usize];
    rand::rng().fill_bytes(&mut random_bytes);

    std::fs::write(path, &random_bytes).map_err(|e| format!("Failed to overwrite file: {}", e))?;

    // Sync to disk before unlinking
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|e| format!("Failed to open for sync: {}", e))?;
    file.sync_all()
        .map_err(|e| format!("Failed to sync to disk: {}", e))?;

    // Remove
    std::fs::remove_file(path).map_err(|e| format!("Failed to delete file: {}", e))?;

    Ok(())
}
