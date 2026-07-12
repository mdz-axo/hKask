#!/usr/bin/env bash
# pipeline.sh — Self-validating QA pipeline with checkpoint/resume.
# Sources corpus/env/pipeline.env. Safe to re-run on interruption.
#
# Usage:
#   ./scripts/pipeline.sh              # Full pipeline
#   ./scripts/pipeline.sh --help       # This message
#   ./scripts/pipeline.sh --skip-gen   # Skip QA generation (use cached)
#   ./scripts/pipeline.sh --dry-run    # Show what would run
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ENV_FILE="${SCRIPT_DIR}/../corpus/env/pipeline.env"
[ -f "$ENV_FILE" ] && . "$ENV_FILE"

# ── Parse flags ─────────────────────────────────────────────────────────
SKIP_GEN=false; DRY_RUN=false
for arg in "$@"; do
    case "$arg" in
        --help|-h)  sed -n '2,14p' "$0"; exit 0 ;;
        --skip-gen) SKIP_GEN=true ;;
        --dry-run)  DRY_RUN=true ;;
        *) echo "Unknown: $arg"; exit 1 ;;
    esac
done

# ── Resolve paths ────────────────────────────────────────────────────────
CORPUS_BIN="${CORPUS_BIN:-target/release/corpus-ingest}"
PROMPTS="${CORPUS_PROMPTS_JSONL:-corpus/qa_pairs/prompts_bloom.jsonl}"
CHECKPOINT_DIR="${CHECKPOINT_DIR:-corpus/qa_pairs/.checkpoint}"
mkdir -p "$CHECKPOINT_DIR"
WORKER_COUNT="${QA_GENERATION_WORKERS:-3}"
CONCURRENCY="${QA_GENERATION_CONCURRENCY:-10}"

step()     { echo ""; echo "=== Step: $1 ==="; }
is_done()  { [ -f "$CHECKPOINT_DIR/$1.done" ]; }
mark_done(){ touch "$CHECKPOINT_DIR/$1.done"; }
dry()      { $DRY_RUN && echo "  [DRY-RUN] $*" && return 0; return 1; }

# ═══════════════════════════════════════════════════════════════════════════
# Step 0: Validate inputs
# ═══════════════════════════════════════════════════════════════════════════
step "0: Validate inputs"
if [ ! -f "$PROMPTS" ]; then
    echo "  ❌ $PROMPTS missing. Run: $CORPUS_BIN build-prompts --cross-reference"
    exit 1
fi
PROMPT_COUNT=$(wc -l < "$PROMPTS")
echo "  ✅ $PROMPT_COUNT prompts  |  binary: $CORPUS_BIN  |  workers: $WORKER_COUNT"

# ═══════════════════════════════════════════════════════════════════════════
# Step 1: QA Generation (parallel workers with PID tracking + resume)
# ═══════════════════════════════════════════════════════════════════════════
step "1: QA Generation"
CHUNK_SIZE=$((PROMPT_COUNT / WORKER_COUNT))

launch_worker() {
    local wid=$1 start=$2 count=$3
    local out="corpus/qa_pairs/gen_bloom_w${wid}.jsonl"
    local pidfile="$CHECKPOINT_DIR/w${wid}.pid"
    local log="/tmp/bloom3x-w${wid}.log"

    # Already done?
    if is_done "gen_w${wid}"; then
        echo "  W${wid}: ✅ $(wc -l < "$out" 2>/dev/null || echo 0) QAs"
        return 0
    fi

    # Already running?
    if [ -f "$pidfile" ] && kill -0 "$(cat "$pidfile")" 2>/dev/null; then
        echo "  W${wid}: 🔄 running ($(wc -l < "$out" 2>/dev/null || echo 0) QAs)"
        return 1
    fi

    dry "Would launch W${wid}" && return 1

    nohup "$CORPUS_BIN" generate-qa \
        "$PROMPTS" --concurrency "$CONCURRENCY" \
        --max-prompts "$count" --resume-at "$start" \
        --output "$out" > "$log" 2>&1 &
    echo $! > "$pidfile"
    echo "  W${wid}: 🚀 PID=$(cat "$pidfile") ($start-$((start+count-1)))"
    return 1
}

ALL_DONE=true
for i in $(seq 1 "$WORKER_COUNT"); do
    start=$(( (i-1) * CHUNK_SIZE + 1 ))
    [ "$i" -eq "$WORKER_COUNT" ] && count=$((PROMPT_COUNT - start + 1)) || count=$CHUNK_SIZE
    launch_worker "$i" "$start" "$count" || ALL_DONE=false
done

if ! $ALL_DONE; then
    echo "  Waiting for $(grep -c . "$CHECKPOINT_DIR"/w*.pid 2>/dev/null || echo 0) workers..."
    while pgrep -f "corpus-ingest generate-qa" > /dev/null 2>&1; do
        total=0
        for i in $(seq 1 "$WORKER_COUNT"); do
            total=$((total + $(wc -l < "corpus/qa_pairs/gen_bloom_w${i}.jsonl" 2>/dev/null || echo 0)))
        done
        echo "  [$(date +%H:%M:%S)] $total QAs"
        sleep "${POLL_INTERVAL_SECS:-30}"
    done
fi

for i in $(seq 1 "$WORKER_COUNT"); do mark_done "gen_w${i}"; done
TOTAL_QAS=0
for i in $(seq 1 "$WORKER_COUNT"); do
    TOTAL_QAS=$((TOTAL_QAS + $(wc -l < "corpus/qa_pairs/gen_bloom_w${i}.jsonl" 2>/dev/null || echo 0)))
done
echo "  ✅ Generation done: $TOTAL_QAS QAs"

# ═══════════════════════════════════════════════════════════════════════════
# Step 2: Merge + Transform (with schema validation)
# ═══════════════════════════════════════════════════════════════════════════
step "2: Merge & Transform"
if ! is_done "merge"; then
    dry "Would merge & transform" && exit 0

    MERGED="corpus/qa_pairs/bloom_merged.jsonl"
    > "$MERGED"
    for i in $(seq 1 "$WORKER_COUNT"); do
        f="corpus/qa_pairs/gen_bloom_w${i}.jsonl"
        [ -f "$f" ] && cat "$f" >> "$MERGED"
    done
    echo "  Merged: $(wc -l < "$MERGED") QAs"

    FLAT="${FLAT:-corpus/qa_pairs/bloom_flat.jsonl}"
    python3 scripts/transform_generated_qas.py "$MERGED" "$FLAT"

    # Schema validation: verify flat file structure
    python3 << 'PYEOF'
import json, sys
with open("corpus/qa_pairs/bloom_flat.jsonl") as f:
    first = json.loads(f.readline())
required = ["instruction", "output", "type"]
missing = [k for k in required if k not in first]
if missing:
    print(f"  ❌ Schema validation FAILED: missing fields {missing}")
    sys.exit(1)
actual_type = first.get("type", "")
if actual_type not in ("factual","conceptual","analyze","evaluate","create"):
    print(f"  ⚠️  Unexpected type: '{actual_type}' — continuing anyway")
print(f"  ✅ Schema OK: {list(first.keys())}")
PYEOF
    mark_done "merge"
else
    echo "  ✅ Already merged"
fi

# ═══════════════════════════════════════════════════════════════════════════
# Step 3: Quality Selection + Split (balanced Bloom, 80/10/10)
# ═══════════════════════════════════════════════════════════════════════════
step "3: Quality Selection & Split"
if ! is_done "select"; then
    dry "Would select & split" && exit 0

    FLAT="${FLAT:-corpus/qa_pairs/bloom_flat.jsonl}"
    TARGET="${TARGET_QAS_PER_LEVEL:-1000}"
    MIN_INST="${MIN_INSTRUCTION_LENGTH:-30}"
    MIN_OUT="${MIN_OUTPUT_LENGTH:-50}"
    SEED="${RANDOM_SEED:-42}"
    SPROMPT="${SYSTEM_PROMPT:-You are a Business and Economics Researcher. You analyze the gap between organizational capabilities and actual performance using economic theory, systems thinking, computing principles, scientific method, and institutional analysis. Your core question: What is the economic significance of unrealized potential?}"

    python3 << PYEOF
import json, random
from collections import defaultdict
random.seed($SEED)

qas, skipped, type_mismatch = [], 0, 0
for line in open('$FLAT'):
    d = json.loads(line)
    if d.get('instruction') and d.get('output') \
       and len(d['instruction']) >= $MIN_INST and len(d['output']) >= $MIN_OUT:
        # Validate Bloom type consistency
        t = d.get('type', '')
        if t not in ('factual','conceptual','analyze','evaluate','create',''):
            type_mismatch += 1
        qas.append(d)
    else:
        skipped += 1
print(f'  Loaded: {len(qas)} QAs (skipped {skipped})')
if type_mismatch:
    print(f'  ⚠️  {type_mismatch} QAs have unexpected type values')

qas.sort(key=lambda x: len(x['output']), reverse=True)
by_level = defaultdict(list)
for q in qas:
    by_level[q.get('type', 'unknown')].append(q)

target = $TARGET
selected = []
for level in ['factual', 'conceptual', 'analyze', 'evaluate', 'create']:
    avail = len(by_level[level])
    taken = by_level[level][:target]
    selected.extend(taken)
    status = '✅' if avail >= target else '⚠️'
    print(f'  {status} {level}: {len(taken)}/{target} (had {avail})')

random.shuffle(selected)
s80 = int(len(selected) * 0.80)
s90 = int(len(selected) * 0.90)
splits = {'train': selected[:s80], 'val': selected[s80:s90], 'test': selected[s90:]}

sp = """$SPROMPT"""
for name, qa_list in splits.items():
    path = f'corpus/qa_pairs/{name}_chat.jsonl'
    with open(path, 'w') as f:
        for q in qa_list:
            chat = {'messages': [
                {'role': 'system', 'content': sp},
                {'role': 'user', 'content': q['instruction']},
                {'role': 'assistant', 'content': q['output']}
            ]}
            f.write(json.dumps(chat) + '\n')
    print(f'  {name}: {len(qa_list)} → {path}')

total = len(selected)
chars = sum(len(q['instruction'])+len(q['output']) for q in selected)
print(f'  Total: {total} QAs, ~{chars*0.3:,.0f} tokens')

# Concept coverage analysis
from collections import Counter
concept_counts = Counter()
for q in selected:
    for concept in q.get('concepts', []):
        concept_counts[concept.lower().strip()] += 1
top_concepts = concept_counts.most_common(20)
print(f'  Top concepts (of {len(concept_counts)} unique):')
for concept, count in top_concepts[:10]:
    print(f'    {concept}: {count}')
# Check for missing critical concepts
critical = ['competitive advantage','return on capital','cost of capital','discounted cash flow','margin of safety','valuation','economic profit','capital allocation','management quality']
missing = [c for c in critical if c not in concept_counts]
if missing:
    print(f'  ⚠️  Missing critical concepts: {", ".join(missing)}')
else:
    print(f'  ✅ All critical investment concepts covered')
PYEOF
    mark_done "select"
else
    echo "  ✅ Already selected"
fi

# ═══════════════════════════════════════════════════════════════════════════
# Step 4: Ingest into memory DB
# ═══════════════════════════════════════════════════════════════════════════
step "4: Ingest"
if ! is_done "ingest"; then
    dry "Would ingest" && exit 0
    "$CORPUS_BIN" ingest-qa \
        --output /dev/null \
        --db-path "${CORPUS_MEMORY_DB:-corpus/memory/corpus_memory.db}" \
        --passphrase "${CORPUS_PASSPHRASE:-hkask-default-passphrase-2024}" \
        --embed-qas \
        "${FLAT:-corpus/qa_pairs/bloom_flat.jsonl}" 2>&1 | \
        grep -E 'Raw|Quality|Deduped|Types|Stored|Embedded' || true
    mark_done "ingest"
else
    echo "  ✅ Already ingested"
fi

# ═══════════════════════════════════════════════════════════════════════════
# Step 5: Pre-flight validation (5 gates)
# ═══════════════════════════════════════════════════════════════════════════
step "5: Pre-Flight Validation"
TRAIN_CHAT="corpus/qa_pairs/train_chat.jsonl"
VAL_CHAT="corpus/qa_pairs/val_chat.jsonl"
TEST_CHAT="corpus/qa_pairs/test_chat.jsonl"
FAIL=0

echo "  Gate 1: File existence"
for f in "$TRAIN_CHAT" "$VAL_CHAT" "$TEST_CHAT"; do
    if [ -f "$f" ]; then
        echo "    ✅ $f ($(wc -l < "$f") samples)"
    else
        echo "    ❌ $f MISSING"
        FAIL=1
    fi
done

echo "  Gate 2: Chat format (system+user+assistant)"
python3 << PYEOF
import json, sys
ok = True
for name, path in [("train","$TRAIN_CHAT"),("val","$VAL_CHAT"),("test","$TEST_CHAT")]:
    try:
        with open(path) as f:
            first = json.loads(f.readline())
        roles = [m["role"] for m in first["messages"]]
        assert roles == ["system","user","assistant"], f"Expected [system,user,assistant], got {roles}"
        has_sys = any("Company Research" in m.get("content","") for m in first["messages"] if m["role"]=="system")
        status = "✅" if has_sys else "⚠️ system prompt missing"
        print(f"    {status} {name}: {roles}")
        if not has_sys: ok = False
    except Exception as e:
        print(f"    ❌ {name}: {e}")
        ok = False
sys.exit(0 if ok else 1)
PYEOF
[ $? -ne 0 ] && FAIL=1

echo "  Gate 3: Token length (P95 vs 4096 max)"
python3 << PYEOF
import json
MAX = 4096
lengths = []
with open("$TRAIN_CHAT") as f:
    for line in f:
        d = json.loads(line)
        total = sum(len(m["content"]) for m in d["messages"])
        lengths.append(total * 0.3)
lengths.sort()
n = len(lengths)
p50, p95, p99 = lengths[n//2], lengths[int(n*.95)], lengths[int(n*.99)]
over = sum(1 for l in lengths if l > MAX)
pct = (over/n)*100 if n else 0
print(f"    Samples: {n}  P50: {p50:.0f}  P95: {p95:.0f}  P99: {p99:.0f}")
print(f"    Over {MAX}: {over} ({pct:.1f}%)  → {'✅ OK' if pct<5 else '❌ TOO MANY'}")
if pct >= 5: sys.exit(1)
PYEOF
[ $? -ne 0 ] && FAIL=1

echo "  Gate 4: Bloom level distribution (via flat file)"
python3 << PYEOF
import json
from collections import Counter
try:
    levels = Counter()
    with open("${FLAT:-corpus/qa_pairs/bloom_flat.jsonl}") as f:
        for line in f:
            d = json.loads(line)
            levels[d.get('type','unknown')] += 1
    total = sum(levels.values())
    for l in ['factual','conceptual','analyze','evaluate','create']:
        c = levels.get(l,0)
        pct = (c/total*100) if total else 0
        bar = '#' * int(pct/2)
        print(f"    {l:12s}: {c:5d} ({pct:5.1f}%) {bar}")
    # Check balance: no level should be <10% of total
    min_pct = min(levels.get(l,0) for l in ['factual','conceptual','analyze','evaluate','create']) / max(total,1) * 100
    if min_pct < 10:
        print(f"    ⚠️  Min level at {min_pct:.1f}% — consider regenerating")
    else:
        print(f"    ✅ All levels >10%")
except Exception as e:
    print(f"    ⚠️  Could not check: {e}")
PYEOF

echo "  Gate 5: Assistant content quality"
python3 << PYEOF
import json
empty = short = total = 0
with open("$TRAIN_CHAT") as f:
    for line in f:
        for m in json.loads(line)["messages"]:
            if m["role"] != "assistant": continue
            c = m.get("content","").strip()
            if not c: empty += 1
            elif len(c) < 50: short += 1
            total += 1
print(f"    Assistant turns: {total}  Empty: {empty}  Short: {short}")
if empty: print("    ❌ Empty responses found!"); sys.exit(1)
print(f"    ✅ All responses have content")
PYEOF
[ $? -ne 0 ] && FAIL=1

echo ""
if [ $FAIL -eq 0 ]; then
    echo "  ╔══════════════════════════════════════╗"
    echo "  ║  ALL 5 GATES PASSED ✅              ║"
    echo "  ║  Dataset ready for training.         ║"
    echo "  ╚══════════════════════════════════════╝"
else
    echo "  ╔══════════════════════════════════════╗"
    echo "  ║  ${FAIL} GATE(S) FAILED ❌               ║"
    echo "  ║  Fix issues above before training.   ║"
    echo "  ╚══════════════════════════════════════╝"
    exit 1
fi

# ═══════════════════════════════════════════════════════════════════════════
echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║ Pipeline Complete                                        ║"
for split in train val test; do
    f="corpus/qa_pairs/${split}_chat.jsonl"
    printf "║  %-5s: %5d QAs → %-35s ║\n" "$split" "$(wc -l < "$f" 2>/dev/null || echo 0)" "$f"
done
echo "╠══════════════════════════════════════════════════════════╣"
echo "║  Submit to RunPod: ./scripts/runpod_startup.sh           ║"
echo "╚══════════════════════════════════════════════════════════╝"
