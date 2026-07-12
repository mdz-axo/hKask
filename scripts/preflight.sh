#!/usr/bin/env bash
# preflight.sh — Validate training dataset before submitting to RunPod
# Usage: . corpus/env/pipeline.env && ./scripts/preflight.sh
set -euo pipefail

echo "=== Pre-Flight Validation ==="
FAIL=0

# Gate 1: File existence
echo ""
echo "Gate 1: File existence"
for f in "$OUTPUT_TRAIN_CHAT" "$OUTPUT_VAL_CHAT" "$OUTPUT_TEST_CHAT"; do
    if [ -f "$f" ]; then
        lines=$(wc -l < "$f")
        echo "  ✅ $f ($lines samples)"
    else
        echo "  ❌ $f MISSING"
        FAIL=1
    fi
done

# Gate 2: Dataset format (messages structure with system prompt)
echo ""
echo "Gate 2: Dataset format (messages with system + user + assistant)"
python3 << 'PYEOF'
import json, sys
files = [
    ("$OUTPUT_TRAIN_CHAT", "train"),
    ("$OUTPUT_VAL_CHAT", "val"),
    ("$OUTPUT_TEST_CHAT", "test"),
]
ok = True
for path, name in files:
    try:
        with open(path) as f:
            first = json.loads(f.readline())
        msgs = first.get("messages", [])
        roles = [m["role"] for m in msgs]
        if roles == ["system", "user", "assistant"]:
            has_system = any("Business and Economics Research" in m.get("content","") for m in msgs if m["role"]=="system")
            print(f"  ✅ {name}: {roles} format, system={'present' if has_system else 'MISSING'}")
            if not has_system:
                ok = False
        else:
            print(f"  ❌ {name}: unexpected roles: {roles}")
            ok = False
    except Exception as e:
        print(f"  ❌ {name}: {e}")
        ok = False
sys.exit(0 if ok else 1)
PYEOF
[ $? -ne 0 ] && FAIL=1

# Gate 3: Token length analysis
echo ""
echo "Gate 3: Token length (max 4096, check P95)"
python3 << 'PYEOF'
import json
MAX_SEQ = 4096
files = ["$OUTPUT_TRAIN_CHAT"]
lengths = []
for path in files:
    with open(path) as f:
        for line in f:
            d = json.loads(line)
            total = sum(len(m["content"]) for m in d.get("messages", []))
            lengths.append(total * 0.3)  # rough token estimate

lengths.sort()
n = len(lengths)
p50 = lengths[n//2]
p95 = lengths[int(n*0.95)]
p99 = lengths[int(n*0.99)]
over = sum(1 for l in lengths if l > MAX_SEQ)
pct_over = (over/n)*100 if n > 0 else 0

print(f"  Samples: {n}")
print(f"  P50: {p50:.0f} tokens, P95: {p95:.0f}, P99: {p99:.0f}")
print(f"  Over {MAX_SEQ}: {over} ({pct_over:.1f}%)")

if pct_over > 5:
    print(f"  ❌ {pct_over:.1f}% samples exceed max_seq_length")
    sys.exit(1)
elif p95 > MAX_SEQ * 0.8:
    print(f"  ⚠️  P95 at {p95/MAX_SEQ*100:.0f}% of max — consider increasing max_seq_length")
else:
    print(f"  ✅ P95 well within limit ({p95/MAX_SEQ*100:.0f}% of {MAX_SEQ})")
PYEOF
[ $? -ne 0 ] && FAIL=1

# Gate 4: Bloom distribution
echo "Gate 4: Bloom level distribution (via flat file)"
python3 << 'PYEOF'
import json
from collections import Counter
try:
    levels = Counter()
    with open("corpus/qa_pairs/bloom_flat.jsonl") as f:
        for line in f:
            d = json.loads(line)
            levels[d.get('type','unknown')] += 1
    total = sum(levels.values())
    for l in ['factual','conceptual','analyze','evaluate','create']:
        c = levels.get(l,0)
        pct = (c/total*100) if total else 0
        print(f"  {l:12s}: {c:5d} ({pct:5.1f}%)")
    min_pct = min(levels.get(l,0) for l in ['factual','conceptual','analyze','evaluate','create']) / max(total,1) * 100
    if min_pct < 10:
        print(f"  ⚠️  Min level at {min_pct:.1f}% — may be imbalanced")
    else:
        print(f"  ✅ All levels >10%")
except Exception as e:
    print(f"  ⚠️  Could not check: {e}")
PYEOF

# Gate 5: Output quality (no empty assistant content)
echo ""
echo "Gate 5: Output quality (assistant content)"
python3 << 'PYEOF'
import json
empty = 0
short = 0
total = 0
for path in ["$OUTPUT_TRAIN_CHAT"]:
    with open(path) as f:
        for line in f:
            d = json.loads(line)
            for m in d.get("messages", []):
                if m["role"] == "assistant":
                    content = m.get("content", "")
                    if not content.strip():
                        empty += 1
                    elif len(content) < 50:
                        short += 1
                    total += 1

print(f"  Assistant turns: {total}")
print(f"  Empty: {empty}, Short (<50 chars): {short}")
if empty > 0:
    print(f"  ❌ {empty} empty assistant responses")
    sys.exit(1)
elif short > total * 0.1:
    print(f"  ⚠️  {short/total*100:.0f}% responses are short")
else:
    print(f"  ✅ All assistant responses have content")
PYEOF
[ $? -ne 0 ] && FAIL=1

echo ""
if [ $FAIL -eq 0 ]; then
    echo "=== ALL GATES PASSED ✅ ==="
    echo "Dataset ready for training."
else
    echo "=== SOME GATES FAILED ❌ ==="
    echo "Fix issues above before training."
    exit 1
fi
