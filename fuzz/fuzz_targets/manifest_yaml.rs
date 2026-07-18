//! Fuzz target: manifest YAML parser.
//!
//! Feeds arbitrary bytes to `hkask_templates::manifest_loader::load_manifest_from_yaml`.
//! The target passes as long as parsing does not panic — parse errors are
//! expected and swallowed.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Manifests are text; lossy-convert arbitrary bytes to a string.
    let yaml = String::from_utf8_lossy(data);
    // Target passes if this never panics, regardless of input.
    let _ = hkask_templates::load_manifest_from_yaml(&yaml);
});
