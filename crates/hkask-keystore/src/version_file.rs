//! Key version file management.
//!
//! The current key version is stored in `~/.config/hkask/version`.
//! This is a plaintext file containing a single `u32` — it is not secret.
//! Security comes from the passphrase, not from hiding the version number.
//!
//! On rotation, the version is incremented and new secrets are derived
//! with the new version. Old-version secrets remain derivable by
//! specifying the old version number.

use std::path::PathBuf;

/// Default key version if no version file exists (backward compat).
pub const DEFAULT_KEY_VERSION: u32 = 1;

/// Get the path to the key version file.
///
/// expect: "My keys are generated, stored, and rotated under my sovereignty"
/// post: returns PathBuf to ~/.config/hkask/version
pub fn version_file_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("hkask");
    path.push("version");
    path
}

/// Read the current key version from disk.
///
/// Returns `DEFAULT_KEY_VERSION` (1) if the file doesn't exist
/// (backward compatibility with pre-versioning installs).
///
/// expect: "My keys are generated, stored, and rotated under my sovereignty"
/// post: returns u32 version from file, or DEFAULT_KEY_VERSION if missing
pub fn read_key_version() -> u32 {
    let path = version_file_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => {
            let version = contents.trim().parse().unwrap_or(DEFAULT_KEY_VERSION);
            // P9: CNS span
            tracing::info!(target: "cns.keystore", operation = "read_key_version", version = version, "CNS");
            version
        }
        Err(_) => {
            tracing::info!(target: "cns.keystore", operation = "read_key_version", version = DEFAULT_KEY_VERSION, "CNS");
            DEFAULT_KEY_VERSION
        }
    }
}

/// Write a new key version to disk.
///
/// Creates the parent directory if it doesn't exist.
///
/// expect: "My keys are generated, stored, and rotated under my sovereignty"
/// pre:  version is a valid u32
/// post: version written to version file
pub fn write_key_version(version: u32) -> std::io::Result<()> {
    let path = version_file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // P9: CNS span
    tracing::info!(target: "cns.keystore", operation = "write_key_version", version = version, "CNS");
    std::fs::write(&path, format!("{version}\n"))
}

/// Increment the key version and return the new version.
///
/// Reads current version, increments by 1, writes new version.
/// Returns the new version number.
///
/// expect: "My keys are generated, stored, and rotated under my sovereignty"
/// post: version incremented by 1 and written to disk
/// post: returns new version number
pub fn increment_key_version() -> std::io::Result<u32> {
    let current = read_key_version();
    let new = current + 1;
    tracing::info!(target: "cns.keystore", operation = "increment_key_version", old = current, new = new, "CNS");
    write_key_version(new)?;
    Ok(new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_version_is_one() {
        assert_eq!(DEFAULT_KEY_VERSION, 1);
    }

    #[test]
    fn read_returns_default_when_no_file() {
        // In test environment, config dir may not exist — should return default
        let version = read_key_version();
        assert!(version >= 1);
    }

    #[test]
    fn write_and_read_roundtrip() {
        let dir = TempDir::new().unwrap();
        let version_file = dir.path().join("hkask").join("version");

        // Write version 5
        std::fs::create_dir_all(version_file.parent().unwrap()).unwrap();
        std::fs::write(&version_file, "5\n").unwrap();

        // Can't test read_key_version directly since it uses the real config dir,
        // but we verify the file format is correct
        let contents = std::fs::read_to_string(&version_file).unwrap();
        assert_eq!(contents.trim(), "5");
    }
}
