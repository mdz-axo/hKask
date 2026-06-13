#!/bin/bash
set -e
cd "$(dirname "$0")/.."
export HKASK_DB_PASSPHRASE=test-pass
target/debug/kask embed-corpus run \
  --config registry/styles/ulysses-s-twain/corpus.yaml \
  --db /tmp/hkask-test-styles.db \
  --passphrase test-pass \
  2>&1 | tee /tmp/embed-twain.log
