#!/bin/bash
# Embed style replicator corpora via Cloudflare Workers AI (through Okapi)
# Usage: bash embed-mashups.sh [twain|wilde|hemingway|woolf|all]
#
# Environment:
#   OKAPI_BASE_URL  — Okapi server URL (default: http://127.0.0.1:11435)
#   OKAPI_API_KEY   — Okapi API key (required for Cloudflare routing)
#
# Okapi routes the "cf/" model prefix to Cloudflare Workers AI.
# Without an API key, requests fall back to Ollama format and the
# cf/ prefix won't be recognized.

set -e
cd "$(dirname "$0")"
export HKASK_DB_PASSPHRASE=test-pass

DB="${HKASK_DB_PATH:-/tmp/hkask-test-styles.db}"
KASK="target/debug/kask"
OKAPI_URL="${OKAPI_BASE_URL:-http://127.0.0.1:11435}"

embed_one() {
    local name="$1"
    local config="registry/styles/${name}/corpus.yaml"
    echo "=== Embedding ${name} ==="
    echo "Started at $(date)"
    echo "Okapi: ${OKAPI_URL}"
    echo "Model: cf/qwen3-embedding:0.6b (Cloudflare Workers AI via Okapi)"
    $KASK embed-corpus run \
        --config "$config" \
        --db "$DB" \
        --passphrase test-pass \
        --okapi-url "${OKAPI_URL}" 2>&1
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
        echo "Test with:"
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
