#!/bin/bash
# Add expect: fields to all REQ contracts in hkask-storage/src/*.rs
set -euo pipefail

STORAGE_DIR="crates/hkask-storage/src"

# Compute expectation from contract ID
get_expectation() {
    local contract_id="$1"
    local is_test="$2"  # "yes" or "no"

    if [ "$is_test" = "yes" ]; then
        # Extract principle from contract ID prefix
        local principle
        principle=$(echo "$contract_id" | sed -n 's/^\(P[0-9]\{1,2\}\)-.*/\1/p')
        if [ -z "$principle" ]; then
            principle=$(echo "$contract_id" | sed -n 's/^DEP-[0-9]*.*\(P[0-9]\{1,2\}\).*/\1/p')
        fi
        [ -z "$principle" ] && principle="P3"
        echo "\"Storage operation works correctly under test conditions\" [${principle}]"
        return
    fi

    # Extract principle from contract ID
    local principle
    principle=$(echo "$contract_id" | sed -n 's/^\(P[0-9]\{1,2\}\)-.*/\1/p')

    # For DEP contracts, extract principle from the REQ line context
    if [ -z "$principle" ]; then
        principle=$(echo "$contract_id" | sed -n 's/.*\(P[0-9]\{1,2\}\).*/\1/p')
        [ -z "$principle" ] && principle="P3"
    fi

    case "$contract_id" in
        # P1 contracts
        P1-sto-user-*|P1-sto-sovereignty-*)
            echo "\"My user data and sovereignty boundaries are stored under my control\" [P1]"
            ;;
        # P2 contracts
        P2-sto-consent-*)
            echo "\"My consent records are stored with explicit affirmative consent\" [P2]"
            ;;
        # P4 contracts
        P4-sto-*)
            echo "\"The system enforces OCAP boundaries on storage access\" [P4]"
            ;;
        # P8 contracts
        P8-sto-*)
            echo "\"Storage types preserve semantic identity across operations\" [P8]"
            ;;
        # DEP contracts - use principle from contract text
        DEP-*)
            case "$principle" in
                P1) echo "\"My user data and sovereignty boundaries are stored under my control\" [P1]" ;;
                P5) echo "\"The system provides durable storage for migration data\" [P5]" ;;
                P7) echo "\"The system provides durable storage for evolutionary data\" [P7]" ;;
                *)   echo "\"The system provides durable storage for archival data\" [P3]" ;;
            esac
            ;;
        # P3 contracts - extract domain
        P3-sto-agent-registry-*)
            echo "\"The system provides durable storage for agent registry data\" [P3]"
            ;;
        P3-sto-triple-*)
            echo "\"The system provides durable storage for triple data\" [P3]"
            ;;
        P3-sto-embedding-*)
            echo "\"The system provides durable storage for embedding data\" [P3]"
            ;;
        P3-sto-gallery-*)
            echo "\"The system provides durable storage for gallery data\" [P3]"
            ;;
        P3-sto-goal-*)
            echo "\"The system provides durable storage for goal data\" [P3]"
            ;;
        P3-sto-wallet-*)
            echo "\"The system provides durable storage for wallet data\" [P3]"
            ;;
        P3-sto-kata-*)
            echo "\"The system provides durable storage for kata history data\" [P3]"
            ;;
        P3-sto-escalation-*)
            echo "\"The system provides durable storage for escalation data\" [P3]"
            ;;
        P3-sto-nu-event-*)
            echo "\"The system provides durable storage for event data\" [P3]"
            ;;
        P3-sto-spec-*)
            echo "\"The system provides durable storage for spec data\" [P3]"
            ;;
        # P9 contracts
        P9-sto-*)
            echo "\"The system provides durable storage for homeostatic data\" [P9]"
            ;;
        # P7 contracts
        P7-sto-*)
            echo "\"The system provides durable storage for evolutionary data\" [P7]"
            ;;
        *)
            echo "\"The system provides durable storage for data\" [P3]"
            ;;
    esac
}

for file in "$STORAGE_DIR"/*.rs; do
    basename=$(basename "$file")
    echo "Processing: $basename"

    tempfile=$(mktemp)
    in_test_module="no"
    prev_line=""

    while IFS= read -r line || [ -n "$line" ]; do
        # Track whether we're in a test module
        if echo "$line" | grep -q '^\s*mod tests\s*{'; then
            in_test_module="yes"
        fi

        # Write the previous line (and insert expect if needed)
        if [ -n "$prev_line" ]; then
            echo "$prev_line" >> "$tempfile"

            # Check if prev_line is a REQ line
            if echo "$prev_line" | grep -q '^[[:space:]]*/// REQ:'; then
                # Extract contract ID - everything after "/// REQ: " up to the next space or em-dash or end
                contract_id=$(echo "$prev_line" | sed -n 's/^[[:space:]]*\/\/\/ REQ: \([^- ]*\).*/\1/p')
                if [ -z "$contract_id" ]; then
                    contract_id=$(echo "$prev_line" | sed -n 's/^[[:space:]]*\/\/\/ REQ: \([^ ]*\).*/\1/p')
                fi

                # Determine if this is a test contract
                is_test="no"
                if [ "$in_test_module" = "yes" ]; then
                    is_test="yes"
                fi
                # Also check if the contract ID itself ends with -test (for function-doc test markers)
                # but only in test module context

                if [ -n "$contract_id" ]; then
                    expectation=$(get_expectation "$contract_id" "$is_test")
                    # Calculate the indentation from the REQ line
                    indent=$(echo "$prev_line" | sed -n 's/^\([[:space:]]*\).*/\1/p')
                    echo "${indent}/// expect: ${expectation}" >> "$tempfile"
                fi
            fi
        fi

        prev_line="$line"
    done < "$file"

    # Don't forget the last line
    echo "$prev_line" >> "$tempfile"
    # Check last line for REQ
    if echo "$prev_line" | grep -q '^[[:space:]]*/// REQ:'; then
        contract_id=$(echo "$prev_line" | sed -n 's/^[[:space:]]*\/\/\/ REQ: \([^- ]*\).*/\1/p')
        if [ -z "$contract_id" ]; then
            contract_id=$(echo "$prev_line" | sed -n 's/^[[:space:]]*\/\/\/ REQ: \([^ ]*\).*/\1/p')
        fi
        is_test="no"
        [ "$in_test_module" = "yes" ] && is_test="yes"
        if [ -n "$contract_id" ]; then
            expectation=$(get_expectation "$contract_id" "$is_test")
            indent=$(echo "$prev_line" | sed -n 's/^\([[:space:]]*\).*/\1/p')
            echo "${indent}/// expect: ${expectation}" >> "$tempfile"
        fi
    fi

    mv "$tempfile" "$file"
done

echo "Done. Running cargo check..."
