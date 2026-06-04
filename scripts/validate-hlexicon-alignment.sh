#!/bin/bash
# hLexicon Alignment Validation Script
# ℏKask v0.22.0 — Validate functional/implementation alignment

set -e

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REGISTRY_DIR="$WORKSPACE_ROOT/registry"
REPORT_FILE="$WORKSPACE_ROOT/docs/architecture/hlexicon-validation-report.md"

echo "🔍 hLexicon Alignment Validation"
echo "================================"
echo ""

# Initialize counters
WORDACT_COUNT=0
FLOWDEF_COUNT=0
KNOWACT_COUNT=0
MISSING_ROLE=0
TOTAL_TEMPLATES=0

# Check Jinja2 templates
echo "📄 Checking Jinja2 templates..."
for template in $(find "$REGISTRY_DIR/templates" -name "*.j2" 2>/dev/null); do
    TOTAL_TEMPLATES=$((TOTAL_TEMPLATES + 1))

    # Check for functional_role in header comment (format: {# functional_role: xxx #})
    if grep -q "{# functional_role:" "$template" 2>/dev/null; then
        ROLE=$(grep "{# functional_role:" "$template" | head -1 | sed 's/.*functional_role: *\([a-z]*\).*/\1/')
        case "$ROLE" in
            wordact) WORDACT_COUNT=$((WORDACT_COUNT + 1)) ;;
            flowdef) FLOWDEF_COUNT=$((FLOWDEF_COUNT + 1)) ;;
            knowact) KNOWACT_COUNT=$((KNOWACT_COUNT + 1)) ;;
            *) MISSING_ROLE=$((MISSING_ROLE + 1)) ;;
        esac
    else
        MISSING_ROLE=$((MISSING_ROLE + 1))
        echo "  ⚠️  Missing functional_role: $template"
    fi
done

# Check YAML manifests
echo "📋 Checking YAML manifests..."
for manifest in $(find "$REGISTRY_DIR/manifests" -name "*.yaml" 2>/dev/null); do
    TOTAL_TEMPLATES=$((TOTAL_TEMPLATES + 1))

    # Check for functional_role in manifest metadata
    if grep -q "functional_role:" "$manifest" 2>/dev/null; then
        ROLE=$(grep "functional_role:" "$manifest" | head -1 | awk '{print $2}')
        case "$ROLE" in
            wordact) WORDACT_COUNT=$((WORDACT_COUNT + 1)) ;;
            flowdef) FLOWDEF_COUNT=$((FLOWDEF_COUNT + 1)) ;;
            knowact) KNOWACT_COUNT=$((KNOWACT_COUNT + 1)) ;;
            *) MISSING_ROLE=$((MISSING_ROLE + 1)) ;;
        esac
    else
        # Manifests without functional_role are assumed FlowDef (orchestration)
        FLOWDEF_COUNT=$((FLOWDEF_COUNT + 1))
    fi
done

# Check YAML ports
echo "🔌 Checking YAML ports..."
for port_file in $(find "$REGISTRY_DIR/ports" -name "*.yaml" 2>/dev/null); do
    # Ports are counted separately
    echo "  Found port file: $port_file"
done

# Calculate totals
TOTAL_COUNTED=$((WORDACT_COUNT + FLOWDEF_COUNT + KNOWACT_COUNT))
COMPLIANCE_RATE=0
if [ $TOTAL_TEMPLATES -gt 0 ]; then
    COMPLIANCE_RATE=$(( (TOTAL_COUNTED * 100) / TOTAL_TEMPLATES ))
fi

# Check distribution balance
echo ""
echo "📊 Functional Distribution:"
echo "  WordAct: $WORDACT_COUNT ($(( WORDACT_COUNT * 100 / (TOTAL_COUNTED + 1) ))%)"
echo "  FlowDef: $FLOWDEF_COUNT ($(( FLOWDEF_COUNT * 100 / (TOTAL_COUNTED + 1) ))%)"
echo "  KnowAct: $KNOWACT_COUNT ($(( KNOWACT_COUNT * 100 / (TOTAL_COUNTED + 1) ))%)"
echo ""

# Warn if distribution is skewed
SKEWED=false
for count in $WORDACT_COUNT $FLOWDEF_COUNT $KNOWACT_COUNT; do
    if [ $TOTAL_COUNTED -gt 0 ]; then
        PERCENT=$(( count * 100 / TOTAL_COUNTED ))
        if [ $PERCENT -gt 60 ]; then
            echo "  ⚠️  WARNING: Distribution skewed (>60% in one category)"
            SKEWED=true
        fi
    fi
done

# Generate report
echo ""
echo "📝 Generating validation report..."
cat > "$REPORT_FILE" << EOF
# hLexicon Alignment Validation Report

**Date:** $(date +%Y-%m-%d)
**Version:** v0.22.0

## Summary

| Metric | Value |
|--------|-------|
| Total Templates/Manifests | $TOTAL_TEMPLATES |
| With functional_role | $TOTAL_COUNTED |
| Missing functional_role | $MISSING_ROLE |
| Compliance Rate | ${COMPLIANCE_RATE}% |

## Functional Distribution

| Category | Count | Percentage |
|----------|-------|------------|
| WordAct | $WORDACT_COUNT | $(( WORDACT_COUNT * 100 / (TOTAL_COUNTED + 1) ))% |
| FlowDef | $FLOWDEF_COUNT | $(( FLOWDEF_COUNT * 100 / (TOTAL_COUNTED + 1) ))% |
| KnowAct | $KNOWACT_COUNT | $(( KNOWACT_COUNT * 100 / (TOTAL_COUNTED + 1) ))% |

## Validation Results

$([ $MISSING_ROLE -eq 0 ] && echo "✅ All templates have functional_role declared" || echo "❌ $MISSING_ROLE templates missing functional_role")
$([ $SKEWED = false ] && echo "✅ Distribution balanced (no category >60%)" || echo "⚠️ Distribution skewed")

## Templates Missing functional_role

EOF

# List templates missing functional_role
for template in $(find "$REGISTRY_DIR/templates" -name "*.j2" 2>/dev/null); do
    if ! grep -q "functional_role:" "$template" 2>/dev/null; then
        echo "- \`$template\`" >> "$REPORT_FILE"
    fi
done

echo "" >> "$REPORT_FILE"
echo "## Orthogonal Mapping" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo "Functional logic and implementation logic are orthogonal surfaces." >> "$REPORT_FILE"
echo "See \`docs/architecture/hlexicon-functional-logic-note.md\` for design rationale." >> "$REPORT_FILE"

echo ""
echo "✅ Validation complete!"
echo "   Report: $REPORT_FILE"
echo ""

# Exit with error if compliance < 100%
if [ $COMPLIANCE_RATE -lt 100 ]; then
    echo "⚠️  Compliance rate is ${COMPLIANCE_RATE}% (target: 100%)"
    exit 1
fi

exit 0
