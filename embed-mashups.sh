#!/bin/bash
# Embed style replicator corpora via Cloudflare Workers AI (through Okapi)
# Usage: bash embed-mashups.sh [twain|wilde|hemingway|woolf|all]
#
# Requires Okapi configured with Cloudflare provider routing.
# Set OKAPI_BASE_URL and/or OKAPI_API_KEY if needed.
# The "cf/" model prefix tells Okapi to route to Cloudflare Workers AI.

set -e
cd "$(dirname "$0")"
export HKASK_DB_PASSPHRASE=test-pass
DB="/tmp/hkask-test-styles.db"
KASK="target/debug/kask"

embed_one() {
    local name="$1"
    local config="registry/styles/${name}/corpus.yaml"
    echo "=== Embedding ${name} ==="
    echo "Started at $(date)"
    echo "Model: cf/qwen3-embedding:0.6b (Cloudflare Workers AI via Okapi)"
    $KASK embed-corpus run \
        --config "$config" \
        --db "$DB" \
        --passphrase test-pass 2>&1
    echo "=== ${name} done at $(date) ==="
}

case "${1:-all}" in
    twain)
        embed_one "ulysses-s-twain"
        ;;
    wilde)
        embed_one "jane-wilde"
        ;;
    hemingway)
        embed_one "hemingway"
        ;;
    woolf)
        embed_one "woolf"
        ;;
    all)
        echo "=== Embedding all 4 replicators ==="
        embed_one "hemingway"
        echo ""
        embed_one "woolf"
        echo ""
        embed_one "ulysses-s-twain"
        echo ""
        embed_one "jane-wilde"
        echo ""
        echo "=== All replicators embedded ==="
        echo ""
        echo "Test with:"
        echo "  kask compose run --prompt '...' --cognition registry/registries/cognition/hemingway-style-synthesizer.yaml --db $DB --passphrase test-pass"
        echo "  kask compose run --prompt '...' --cognition registry/registries/cognition/woolf-style-synthesizer.yaml --db $DB --passphrase test-pass"
        echo "  kask compose run --prompt '...' --cognition registry/registries/cognition/ulysses-s-twain-mashup.yaml --db $DB --passphrase test-pass"
        echo "  kask compose run --prompt '...' --cognition registry/registries/cognition/jane-wilde-mashup.yaml --db $DB --passphrase test-pass"
        ;;
esac
