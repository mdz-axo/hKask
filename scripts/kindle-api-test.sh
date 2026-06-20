#!/usr/bin/env bash
# kindle-api-test.sh — Test Kindle Cloud Reader API endpoints
# Sources: kindle-api (Xetera), kindle-ai-export (transitive-bullshit)
# NOTE: Amazon TLS-fingerprints requests. This may fail with 403 without a TLS proxy.
set -euo pipefail

COOKIES="${KINDLE_COOKIES:-}"
if [ -z "$COOKIES" ]; then
    echo "Set KINDLE_COOKIES with your read.amazon.com cookies:"
    echo '  ubid-main, at-main, x-main, session-id'
    exit 1
fi

UA="Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/112.0.0.0 Safari/537.36"

echo "→ Step 1: List books (kindle-api endpoint)"
curl -sS -b "$COOKIES" -H "User-Agent: $UA" \
    "https://read.amazon.com/kindle-library/search?query=&libraryType=BOOKS&sortType=recency&querySize=5" \
    | python3 -c "import sys,json; d=json.load(sys.stdin); [print(f'  {b[\"title\"]:50s} ASIN={b[\"asin\"]}') for b in d.get('itemsList',[])]" 2>/dev/null || echo "  (failed or not JSON — TLS fingerprint may block)"
echo

ASIN="${1:-}"
if [ -z "$ASIN" ]; then
    echo "Pass an ASIN to test render: $0 B0XXXXXXXX"
    exit 0
fi

echo "→ Step 2: startReading (kindle-ai-export endpoint)"
START=$(curl -sS -b "$COOKIES" -H "User-Agent: $UA" \
    "https://read.amazon.com/service/mobile/reader/startReading?asin=$ASIN&clientVersion=20000100" 2>&1)
KARAMEL=$(echo "$START" | python3 -c "import sys,json; print(json.load(sys.stdin).get('karamelToken',{}).get('token',''))" 2>/dev/null)

if [ -z "$KARAMEL" ]; then
    echo "  No karamelToken — response:"
    echo "$START" | head -c 500
    echo
    echo "  NOTE: The renderer/render endpoint returns glyph data (not images)."
    echo "  Page images are rendered by the browser using WebGL — cannot be fetched via API alone."
    exit 1
fi
echo "  karamelToken: ${#KARAMEL} chars"

echo "→ Step 3: renderer/render (returns TAR of glyph data, NOT images)"
HTTP=$(curl -sS -b "$COOKIES" \
    -H "User-Agent: $UA" \
    -H "x-amz-rendering-token: $KARAMEL" \
    "https://read.amazon.com/renderer/render?version=3.0&asin=$ASIN&contentType=FullBook&fontFamily=Bookerly&fontSize=8&lineHeight=1.4&dpi=160&height=800&width=1200&marginBottom=0&marginLeft=9&marginRight=9&marginTop=0&maxNumberColumns=1&packageType=TAR&encryptionVersion=NONE&numPage=1&skipPageCount=0&startingPosition=0&bundleImages=false" \
    -o /tmp/kindle-test.tar -w "%{http_code}" 2>&1)

echo "  HTTP $HTTP"
if [ "$HTTP" = "200" ]; then
    echo "  TAR contents (glyph data, metadata, location maps — no rendered images):"
    tar tf /tmp/kindle-test.tar 2>/dev/null | head -20
    echo
    echo "  VERDICT: API works but returns glyph-DRM data."
    echo "  Rendered page images require a browser (Chrome CDP / Playwright)."
    echo "  This confirms the CDP approach is necessary for visual page capture."
else
    echo "  FAILED — Amazon TLS fingerprinting likely blocking this request."
    echo "  kindle-api solves this with a separate tls-client-api proxy."
fi
