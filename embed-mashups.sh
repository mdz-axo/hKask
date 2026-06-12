#!/bin/bash
# Embed style replicator corpora via hKask's inference engine (DeepInfra)
# Usage: bash embed-mashups.sh [twain|wilde|hemingway|woolf|eliot|all]
#
# Prerequisites:
#   DI_API_KEY must be set in the environment (DeepInfra API key).
#   Corpus configs use model "DI/Qwen/Qwen3-Embedding-0.6B" — the DI/ prefix
#   routes through EmbeddingRouter to DeepInfra's /v1/embeddings endpoint.
#
# Environment:
#   DI_API_KEY      — DeepInfra API key (required)
#   DI_BASE_URL     — DeepInfra base URL (default: https://api.deepinfra.com/v1/openai)
#   HKASK_DB_PATH   — Database path (default: /tmp/hkask-test-styles.db)

set -e
cd "$(dirname "$0")"
export HKASK_DB_PASSPHRASE=test-pass

DB="${HKASK_DB_PATH:-/tmp/hkask-test-styles.db}"

# Prefer debug build, fall back to release
if [ -x "target/debug/kask" ]; then
    KASK="target/debug/kask"
elif [ -x "target/release/kask" ]; then
    KASK="target/release/kask"
else
    echo "ERROR: kask binary not found at target/debug/kask or target/release/kask" >&2
    echo "Build with: cargo build" >&2
    exit 1
fi

if [ -z "${DI_API_KEY:-}" ] && [ -z "${DEEPINFRA_API_KEY:-}" ]; then
    echo "ERROR: Neither DI_API_KEY nor DEEPINFRA_API_KEY is set. DeepInfra API key is required for embedding." >&2
    echo "Get one at https://deepinfra.com/ and export DI_API_KEY=<your-key>" >&2
    exit 1
fi

embed_one() {
    local name="$1"
    local config="registry/styles/${name}/corpus.yaml"
    echo "=== Embedding ${name} ==="
    echo "Started at $(date)"
    echo "Model: DI/Qwen/Qwen3-Embedding-0.6B (DeepInfra)"
    $KASK style embed-corpus \
        --config "$config" \
        --db "$DB" \
        --passphrase test-pass 2>&1
    echo "=== ${name} done at $(date) ==="
}

case "${1:-all}" in
    twain)    embed_one "ulysses-s-twain" ;;
    wilde)    embed_one "jane-wilde" ;;
    hemingway) embed_one "hemingway" ;;
    woolf)    embed_one "woolf" ;;
    eliot)    embed_one "agatha-eliot" ;;
    all)
        echo "=== Embedding all 5 replicators ==="
        for name in hemingway woolf ulysses-s-twain jane-wilde agatha-eliot; do
            embed_one "$name"
            echo ""
        done
        echo "=== All replicators embedded ==="
        echo ""
        echo "Test with replica MCP server tools:"
        echo "  kask pod assign <replicant> replica"
        echo "  kask pod mode <replicant> server -r replica"
        echo ""
        echo "Or via CLI compose:"
        echo "  kask compose run --prompt '...' --cognition registry/registries/cognition/hemingway-style-synthesizer.yaml --db $DB --passphrase test-pass"
        echo "  kask compose run --prompt '...' --cognition registry/registries/cognition/woolf-style-synthesizer.yaml --db $DB --passphrase test-pass"
        echo "  kask compose run --prompt '...' --cognition registry/registries/cognition/ulysses-s-twain-mashup.yaml --db $DB --passphrase test-pass"
        echo "  kask compose run --prompt '...' --cognition registry/registries/cognition/agatha-eliot-mashup.yaml --db $DB --passphrase test-pass"
        ;;
    *)
        echo "Usage: bash embed-mashups.sh [twain|wilde|hemingway|woolf|eliot|all]"
        exit 1
        ;;
esac
