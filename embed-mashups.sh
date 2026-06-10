#!/bin/bash
# Embed both mashup corpora
# Usage: bash embed-mashups.sh [twain|wilde|both]

set -e
cd "$(dirname "$0")"
export HKASK_DB_PASSPHRASE=test-pass
DB="/tmp/hkask-test-styles.db"
KASK="target/debug/kask"

embed_one() {
    local name="$1"
    local config="registry/styles/${name}/corpus.yaml"
    echo "=== Embedding $name ==="
    echo "Started at $(date)"
    $KASK embed-corpus run \
        --config "$config" \
        --db "$DB" \
        --passphrase test-pass 2>&1
    echo "=== $name done at $(date) ==="
}

case "${1:-both}" in
    twain)
        embed_one "ulysses-s-twain"
        ;;
    wilde)
        embed_one "jane-wilde"
        ;;
    both)
        embed_one "ulysses-s-twain"
        echo ""
        embed_one "jane-wilde"
        echo ""
        echo "=== Both mashups embedded ==="
        echo "Test with:"
        echo "  kask compose run --prompt 'Write about...' --cognition registry/registries/cognition/ulysses-s-twain-mashup.yaml --db $DB --passphrase test-pass"
        echo "  kask compose run --prompt 'Write about...' --cognition registry/registries/cognition/jane-wilde-mashup.yaml --db $DB --passphrase test-pass"
        ;;
esac
