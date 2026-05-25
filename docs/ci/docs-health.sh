#!/bin/bash
# hKask Documentation Health Check
# Quick verification of documentation standards
#
# Usage: ./docs/ci/docs-health.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCS_DIR="$(dirname "$SCRIPT_DIR")"

echo "=== hKask Documentation Health Check ==="
echo ""

# Check 1: Architecture master exists
if [[ -f "$DOCS_DIR/architecture/hKask-architecture-master.md" ]]; then
    echo "✓ Architecture master document exists"
else
    echo "✗ Architecture master document missing"
    exit 1
fi

# Check 2: Link checker exists
if [[ -f "$SCRIPT_DIR/check-links.sh" ]]; then
    echo "✓ Link checker script exists"
else
    echo "✗ Link checker script missing"
    exit 1
fi

# Check 3: Metadata checker exists
if [[ -f "$SCRIPT_DIR/check-metadata.sh" ]]; then
    echo "✓ Metadata checker script exists"
else
    echo "✗ Metadata checker script missing"
    exit 1
fi

# Check 4: Core architecture docs have headers
for doc in "PRINCIPLES.md" "DDMVSS.md" "domain-and-capability.md" "interface-and-composition.md" "magna-carta.md" "persistence-and-lifecycle.md" "trust-security-observability.md"; do
    file="$DOCS_DIR/architecture/$doc"
    if [[ -f "$file" ]]; then
        first_line=$(head -n 1 "$file")
        if [[ "$first_line" == "---" ]]; then
            echo "✓ $doc has YAML frontmatter"
        else
            echo "✗ $doc missing YAML frontmatter"
        fi
    else
        echo "✗ $doc not found"
    fi
done

echo ""
echo "=== Summary ==="
echo "Documentation refresh complete"
echo "Run './docs/ci/check-links.sh' to verify all links"
echo "Run './docs/ci/check-metadata.sh' to verify metadata headers"