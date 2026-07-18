//! Fuzz target: minijinja template parser.
//!
//! Feeds arbitrary bytes to `minijinja::Environment::add_template`. The
//! target passes as long as template compilation does not panic —
//! syntax errors are expected and swallowed.

#![no_main]
use libfuzzer_sys::fuzz_target;
use minijinja::Environment;

fuzz_target!(|data: &[u8]| {
    let template = String::from_utf8_lossy(data);
    let mut env = Environment::new();
    // Target passes if this never panics, regardless of input.
    let _ = env.add_template("test", &template);
});
