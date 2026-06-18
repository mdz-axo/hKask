#!/bin/bash
# Add expect: fields to all REQ contracts in hkask-storage/src/*.rs
# Handles ///, //, and //! comment styles
set -euo pipefail

STORAGE_DIR="crates/hkask-storage/src"

# Compute expectation from contract ID and full REQ line
# $1 = contract_id, $2 = is_test, $3 = comment_prefix, $4 = full_req_line (for principle extraction)
get_expectation() {
    local contract_id="$1"
    local is_test="$2"
    local prefix="$3"
    local full_line="$4"

    # Extract principle: try from contract_id first, then from full line, default to P3
    local principle
    principle=$(echo "$contract_id" | sed -n 's/^\(P[0-9]\{1,2\}\)-.*/\1/p')
    if [ -z "$principle" ]; then
        # Try to find P# in the full REQ line (e.g., "DEP-200 — P1 User Sovereignty")
        principle=$(echo "$full_line" | grep -oP 'P\d+' | head -1)
    fi
    [ -z "$principle" ] && principle="P3"

    if [ "$is_test" = "yes" ]; then
        echo "${prefix} expect: \"Storage operation works correctly under test conditions\" [${principle}]"
        return
    fi

    case "$contract_id" in
        P1-sto-user-*|P1-sto-sovereignty-*)
            _expect_body="\"My user data and sovereignty boundaries are stored under my control\" [P1]" ;;
        P2-sto-consent-*)
            _expect_body="\"My consent records are stored with explicit affirmative consent\" [P2]" ;;
        P4-sto-*)
            _expect_body="\"The system enforces OCAP boundaries on storage access\" [P4]" ;;
        P8-sto-*)
            _expect_body="\"Storage types preserve semantic identity across operations\" [P8]" ;;
        P3-sto-agent-registry-*)
            _expect_body="\"The system provides durable storage for agent registry data\" [P3]" ;;
        P3-sto-triple-*)
            _expect_body="\"The system provides durable storage for triple data\" [P3]" ;;
        P3-sto-embedding-*)
            _expect_body="\"The system provides durable storage for embedding data\" [P3]" ;;
        P3-sto-gallery-*)
            _expect_body="\"The system provides durable storage for gallery data\" [P3]" ;;
        P3-sto-goal-*)
            _expect_body="\"The system provides durable storage for goal data\" [P3]" ;;
        P3-sto-wallet-*)
            _expect_body="\"The system provides durable storage for wallet data\" [P3]" ;;
        P3-sto-kata-*)
            _expect_body="\"The system provides durable storage for kata history data\" [P3]" ;;
        P3-sto-escalation-*)
            _expect_body="\"The system provides durable storage for escalation data\" [P3]" ;;
        P3-sto-nu-event-*)
            _expect_body="\"The system provides durable storage for event data\" [P3]" ;;
        P3-sto-spec-*)
            _expect_body="\"The system provides durable storage for spec data\" [P3]" ;;
        P9-sto-*)
            _expect_body="\"The system provides durable storage for homeostatic data\" [P9]" ;;
        P7-sto-*)
            _expect_body="\"The system provides durable storage for evolutionary data\" [P7]" ;;
        DEP-*)
            case "$principle" in
                P1) _expect_body="\"My user data and sovereignty boundaries are stored under my control\" [P1]" ;;
                P5) _expect_body="\"The system provides durable storage for migration data\" [P5]" ;;
                P7) _expect_body="\"The system provides durable storage for evolutionary data\" [P7]" ;;
                *)   _expect_body="\"The system provides durable storage for archival data\" [P3]" ;;
            esac ;;
        *)
            _expect_body="\"The system provides durable storage for data\" [P3]" ;;
    esac

    echo "${prefix} expect: ${_expect_body}"
}

# Extract contract ID from a REQ line
# Contract IDs like P3-sto-* contain dashes, so match up to first space or end of line
extract_contract_id() {
    local line="$1"
    local cid
    # Match non-space chars after "REQ: " — this correctly captures IDs with dashes like P3-sto-triple-insert
    cid=$(echo "$line" | sed -n 's#^[[:space:]]*.*REQ: \([^ ]*\).*#\1#p')
    echo "$cid"
}

# Classify a line: returns "///", "//", "//!", or ""
classify_req() {
    local line="$1"
    if echo "$line" | grep -qE '^[[:space:]]*/// REQ:'; then
        echo "///"
    elif echo "$line" | grep -qE '^[[:space:]]*// REQ:'; then
        echo "//"
    elif echo "$line" | grep -qE '^[[:space:]]*//![[:space:]]*#?[[:space:]]*REQ:'; then
        echo "//!"
    else
        echo ""
    fi
}

# Check if a line is an expect line for a given prefix
is_expect_line() {
    local line="$1"
    local prefix="$2"
    echo "$line" | grep -qE "^[[:space:]]*${prefix} expect:"
}

process_file() {
    local file="$1"
    local basename
    basename=$(basename "$file")
    echo "  $basename"

    local tempfile
    tempfile=$(mktemp)
    local in_test_module="no"
    local prev_line=""

    while IFS= read -r line || [ -n "$line" ]; do
        # Track test module boundaries
        if echo "$line" | grep -qE '^[[:space:]]*mod tests[[:space:]]*\{'; then
            in_test_module="yes"
        fi

        # Process the PREVIOUS line
        if [ -n "$prev_line" ]; then
            echo "$prev_line" >> "$tempfile"

            local req_prefix
            req_prefix=$(classify_req "$prev_line")

            if [ -n "$req_prefix" ]; then
                # Check if next line is already an expect
                if is_expect_line "$line" "$req_prefix"; then
                    : # Already has expect
                else
                    local contract_id
                    contract_id=$(extract_contract_id "$prev_line")
                    if [ -n "$contract_id" ]; then
                        local indent
                        indent=$(echo "$prev_line" | sed 's/^\([[:space:]]*\).*/\1/')
                        local is_test="no"
                        # // REQ: (plain, not ///) is always in test context
                        if [ "$req_prefix" = "//" ] || [ "$in_test_module" = "yes" ]; then
                            is_test="yes"
                        fi
                        local expectation
                        expectation=$(get_expectation "$contract_id" "$is_test" "${indent}${req_prefix}" "$prev_line")
                        echo "$expectation" >> "$tempfile"
                    fi
                fi
            fi
        fi

        prev_line="$line"
    done < "$file"

    # Handle the last line
    if [ -n "$prev_line" ]; then
        echo "$prev_line" >> "$tempfile"

        local req_prefix
        req_prefix=$(classify_req "$prev_line")

        if [ -n "$req_prefix" ]; then
            local contract_id
            contract_id=$(extract_contract_id "$prev_line")
            if [ -n "$contract_id" ]; then
                local indent
                indent=$(echo "$prev_line" | sed 's/^\([[:space:]]*\).*/\1/')
                local is_test="no"
                if [ "$req_prefix" = "//" ] || [ "$in_test_module" = "yes" ]; then
                    is_test="yes"
                fi
                local expectation
                expectation=$(get_expectation "$contract_id" "$is_test" "${indent}${req_prefix}" "$prev_line")
                echo "$expectation" >> "$tempfile"
            fi
        fi
    fi

    mv "$tempfile" "$file"
}

echo "Adding expect: fields to hkask-storage contracts..."
for file in "$STORAGE_DIR"/*.rs; do
    process_file "$file"
done
echo "Done with inserts."
