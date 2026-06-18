#!/usr/bin/env bash
# add-expect-fields.sh — Add expect: fields to all REQ contracts in hkask-services sub-crates.
# Surgical: processes only REQ lines, inserts expect immediately after.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CRATES_DIR="$ROOT_DIR/crates"

# Principle → expectation text mapping
declare -A EXPECT=(
    [P1]='"My data operations flow through sovereignty-verifying service boundaries" [P1]'
    [P2]='"Service operations require explicit consent" [P2]'
    [P3]='"The service layer enables generative access to domain capabilities" [P3]'
    [P4]='"Service boundaries enforce OCAP membranes" [P4]'
    [P5]='"The service layer exposes minimal, essential interfaces shared by all surfaces" [P5]'
    [P7]='"The service interface emerged from real usage patterns" [P7]'
    [P8]='"Service types preserve semantic identity" [P8]'
    [P9]='"The service layer provides CNS health and regulation queries" [P9]'
)

# Process a single .rs file
# For each line matching /// REQ: or // REQ:, insert expect line after it.
process_file() {
    local file="$1"
    local tmpfile="${file}.tmp.$$"

    awk -v p1_exp="${EXPECT[P1]}" \
        -v p2_exp="${EXPECT[P2]}" \
        -v p3_exp="${EXPECT[P3]}" \
        -v p4_exp="${EXPECT[P4]}" \
        -v p5_exp="${EXPECT[P5]}" \
        -v p7_exp="${EXPECT[P7]}" \
        -v p8_exp="${EXPECT[P8]}" \
        -v p9_exp="${EXPECT[P9]}" \
    '
    # Match doc-comment REQ lines: /// REQ: <ID>
    /^[[:space:]]*\/\/\/[[:space:]]*REQ:/ {
        req_id = $0
        sub(/^[[:space:]]*\/\/\/[[:space:]]*REQ:[[:space:]]*/, "", req_id)
        sub(/[[:space:]].*$/, "", req_id)

        principle = "P5"
        if (req_id ~ /^P1-/) principle = "P1"
        else if (req_id ~ /^P2-/) principle = "P2"
        else if (req_id ~ /^P3-/) principle = "P3"
        else if (req_id ~ /^P4-/) principle = "P4"
        else if (req_id ~ /^P5-/) principle = "P5"
        else if (req_id ~ /^P7-/) principle = "P7"
        else if (req_id ~ /^P8-/) principle = "P8"
        else if (req_id ~ /^P9-/) principle = "P9"
        else if (req_id == "P1") principle = "P1"
        else if (req_id == "P2") principle = "P2"
        else if (req_id == "P3") principle = "P3"
        else if (req_id == "P4") principle = "P4"
        else if (req_id == "P7") principle = "P7"
        else if (req_id == "P8") principle = "P8"
        else if (req_id == "P9") principle = "P9"

        prefix = p5_exp
        if (principle == "P1") prefix = p1_exp
        else if (principle == "P2") prefix = p2_exp
        else if (principle == "P3") prefix = p3_exp
        else if (principle == "P4") prefix = p4_exp
        else if (principle == "P5") prefix = p5_exp
        else if (principle == "P7") prefix = p7_exp
        else if (principle == "P8") prefix = p8_exp
        else if (principle == "P9") prefix = p9_exp

        match($0, /^[[:space:]]*/)
        ws = substr($0, RSTART, RLENGTH)

        print $0
        print ws "/// expect: " prefix
        next
    }

    # Match test-comment REQ lines: // REQ: <ID> (but NOT /// or //!)
    /^[[:space:]]*\/\/[[:space:]]*REQ:/ {
        req_id = $0
        sub(/^[[:space:]]*\/\/[[:space:]]*REQ:[[:space:]]*/, "", req_id)
        sub(/[[:space:]].*$/, "", req_id)

        principle = "P5"
        if (req_id ~ /^P1-/) principle = "P1"
        else if (req_id ~ /^P2-/) principle = "P2"
        else if (req_id ~ /^P3-/) principle = "P3"
        else if (req_id ~ /^P4-/) principle = "P4"
        else if (req_id ~ /^P5-/) principle = "P5"
        else if (req_id ~ /^P7-/) principle = "P7"
        else if (req_id ~ /^P8-/) principle = "P8"
        else if (req_id ~ /^P9-/) principle = "P9"
        else if (req_id == "P1") principle = "P1"
        else if (req_id == "P2") principle = "P2"
        else if (req_id == "P3") principle = "P3"
        else if (req_id == "P4") principle = "P4"
        else if (req_id == "P7") principle = "P7"
        else if (req_id == "P8") principle = "P8"
        else if (req_id == "P9") principle = "P9"

        prefix = p5_exp
        if (principle == "P1") prefix = p1_exp
        else if (principle == "P2") prefix = p2_exp
        else if (principle == "P3") prefix = p3_exp
        else if (principle == "P4") prefix = p4_exp
        else if (principle == "P5") prefix = p5_exp
        else if (principle == "P7") prefix = p7_exp
        else if (principle == "P8") prefix = p8_exp
        else if (principle == "P9") prefix = p9_exp

        match($0, /^[[:space:]]*/)
        ws = substr($0, RSTART, RLENGTH)

        print $0
        print ws "// expect: " prefix
        next
    }

    { print }
    ' "$file" > "$tmpfile"

    if ! cmp -s "$file" "$tmpfile"; then
        mv "$tmpfile" "$file"
        echo "  Updated: $file"
    else
        rm "$tmpfile"
    fi
}

# Process all services sub-crates
echo "=== Adding expect: fields to all hkask-services sub-crates ==="
echo ""

TOTAL_CONTRACTS=0
UPDATED_FILES=0

for crate_dir in "$CRATES_DIR"/hkask-services-*/; do
    crate_name=$(basename "$crate_dir")

    req_count=$(grep -rc 'REQ:' "$crate_dir/src/" --include="*.rs" 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
    if [[ "$req_count" -eq 0 ]]; then
        continue
    fi

    echo "--- $crate_name ($req_count contracts) ---"

    while IFS= read -r -d '' file; do
        process_file "$file"
        UPDATED_FILES=$((UPDATED_FILES + 1))
    done < <(grep -rl 'REQ:' "$crate_dir/src/" --include="*.rs" -print0 2>/dev/null || true)

    TOTAL_CONTRACTS=$((TOTAL_CONTRACTS + req_count))
    echo ""
done

echo "=== Summary ==="
echo "Total contracts processed: $TOTAL_CONTRACTS"
echo "Files updated: $UPDATED_FILES"
echo "Done."
