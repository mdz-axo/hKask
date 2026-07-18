//! Fuzz target: capability token deserialization.
//!
//! Feeds arbitrary bytes to `DelegationToken::from_base64` (after lossy
//! conversion to a string) and to `CapabilitySpec::parse`. The target
//! passes as long as neither entry point panics — decode/parse errors
//! are expected and swallowed.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // from_base64 expects a base64 string; lossy-convert arbitrary bytes.
    let candidate = String::from_utf8_lossy(data);
    let _ = hkask_capability::DelegationToken::from_base64(&candidate);

    // Also exercise the capability spec parser on the same input.
    let _ = hkask_capability::CapabilitySpec::parse(&candidate);
});
