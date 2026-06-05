//! Keystore command handlers for `kask keystore`
//!
//! Implements the CLI display logic for key management operations.

use crate::cli::KeystoreAction;

pub fn run(action: KeystoreAction) {
    let keychain = hkask_keystore::Keychain::default();

    match action {
        KeystoreAction::Load {
            path,
            prefix,
            overwrite,
        } => {
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to read {}: {}", path.display(), e);
                    std::process::exit(1);
                }
            };
            let mut loaded = 0usize;
            let mut skipped = 0usize;
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    if !key.starts_with(&prefix) {
                        continue;
                    }
                    if value.is_empty() {
                        continue;
                    }
                    match keychain.retrieve_by_key(key) {
                        Ok(_) if !overwrite => {
                            println!("  skipped {} (already in keychain, use --overwrite)", key);
                            skipped += 1;
                        }
                        _ => match keychain.store_by_key(key, value) {
                            Ok(()) => {
                                println!("  stored {}", key);
                                loaded += 1;
                            }
                            Err(e) => eprintln!("  failed {} : {}", key, e),
                        },
                    }
                }
            }
            println!("\nLoaded {} keys, skipped {}", loaded, skipped);
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
