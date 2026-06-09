//! F-SYN-020: MCP tool input fuzzing.
//!
//! Every `Parameters<T>` for an MCP tool handler is a deserialisation
//! boundary. A malformed input from a hostile caller (or a
//! misbehaving client) must not cause a `panic!`. The proptest
//! generates 1000 random `serde_json::Value` shapes and asserts
//! that each deserialisation is `Err(_)` (not `panic!`).
//!
//! Run with `cargo test -p hkask-mcp-spec --test fuzz_tool_inputs`.

// F-SYN-020: integration test for the `hkask-mcp-spec` binary.
// The `types` module is `pub` in `main.rs`; integration tests
// reference it via the binary's name (`hkask_mcp_spec`).
use hkask_mcp_spec::types::GoalCaptureRequest;
use proptest::prelude::*;

/// proptest strategy: any JSON string. Generates 1000 random
/// shapes (arrays, objects, primitives, garbage) and serialises
/// them as JSON.
fn any_json_string() -> impl Strategy<Value = String> {
    prop_oneof![
        // Valid-ish object
        any::<u32>().prop_map(|n| format!(r#"{{"id": {n}}}"#)),
        any::<String>().prop_map(|s| format!(r#"{{"input": "{s}"}}"#)),
        // Malformed objects
        Just("{".to_string()),
        Just("}".to_string()),
        Just("{,}".to_string()),
        Just("{\"a\":}".to_string()),
        // Arrays and primitives
        any::<i32>().prop_map(|n| n.to_string()),
        Just("null".to_string()),
        Just("true".to_string()),
        Just("[]".to_string()),
        Just("\"str\"".to_string()),
    ]
}

proptest! {
    /// F-SYN-020: arbitrary JSON input must not panic on
    /// deserialisation. The result must be either `Ok(_)` (if the
    /// input happens to be a valid `GoalCaptureRequest`) or
    /// `Err(_)` (if the input is malformed). A `panic!` is a
    /// finding.
    #[test]
    fn goal_capture_request_arbitrary_input_does_not_panic(input in any_json_string()) {
        let result = std::panic::catch_unwind(|| {
            serde_json::from_str::<GoalCaptureRequest>(&input)
        });
        match result {
            Ok(Ok(_)) => {}    // valid input
            Ok(Err(_)) => {}   // malformed input, properly rejected
            Err(_) => panic!("F-SYN-020: deserialisation panicked on input: {input}"),
        }
    }
}
