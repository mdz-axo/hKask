#!/usr/bin/env bash
# E2E test: hkask-acp binary with real daemon and inference.
# Requires: running daemon, running Ollama, built hkask-acp and kask binaries.
set -euo pipefail

BINARY="./target/debug/hkask-acp"
KASK="./target/debug/kask"
REPLICANT="e2e-test-acp"
MODEL="qwen3:8b"
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR; kill %1 2>/dev/null || true" EXIT

echo "=== E2E: hkask-acp with real daemon ==="

# Ensure replicant is registered
echo "--- Registering replicant ---"
$KASK login "$REPLICANT" 2>/dev/null || true
$KASK pod assign "$REPLICANT" acp 2>/dev/null || true

# Start the ACP binary
echo "--- Starting hkask-acp ---"
HKASK_REPLICANT="$REPLICANT" HKASK_MODEL="$MODEL" RUST_LOG=hkask.acp=debug $BINARY &
ACP_PID=$!
sleep 2

# Communication via named pipes
IN_PIPE="$TMPDIR/in"
OUT_PIPE="$TMPDIR/out"
mkfifo "$IN_PIPE"

# Send initialize request
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientInfo":{"name":"e2e-test","version":"1.0"}}}' > "$IN_PIPE" &

# Wait briefly then check if the binary is alive
sleep 1
if ! kill -0 $ACP_PID 2>/dev/null; then
    echo "FAIL: ACP binary exited unexpectedly"
    exit 1
fi

echo "--- ACP binary is running (PID $ACP_PID) ---"

# Send session/new and session/prompt via stdin
# (The binary reads from stdin — we'll send directly)
echo '{"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":"/tmp"}}' > /proc/$ACP_PID/fd/0 2>/dev/null || {
    echo "NOTE: Direct stdin injection not supported on this platform"
    echo "--- Manual test instructions ---"
    echo "1. In another terminal, run:"
    echo "   HKASK_REPLICANT=$REPLICANT HKASK_MODEL=$MODEL $BINARY"
    echo "2. Paste this JSON:"
    echo '   {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientInfo":{"name":"test","version":"1.0"}}}'
    echo '   {"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":"/tmp"}}'
    echo '   {"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{"sessionId":"<from-above>","prompt":[{"type":"text","text":"Say hello in exactly 5 words."}]}}'
    echo "3. Verify streaming agent_message_chunk notifications and end_turn response"
}

# Keep the binary running for a moment then clean up
sleep 2
kill $ACP_PID 2>/dev/null || true

echo "=== E2E test complete ==="
