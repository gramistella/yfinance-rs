#!/usr/bin/env bash
set -euo pipefail

# Use an impersonating curl binary to match yahooquery's TLS/browser fingerprint
# Try Chrome 131 first, fall back to 124 if needed.
CURL_BIN="${CURL_BIN:-}"
if command -v curl_chrome131 >/dev/null 2>&1; then
  CURL_BIN="curl_chrome131"
elif command -v curl_chrome124 >/dev/null 2>&1; then
  CURL_BIN="curl_chrome124"
elif command -v curl_chrome116 >/dev/null 2>&1; then
  CURL_BIN="curl_chrome116"
fi

if [ -z "${CURL_BIN}" ]; then
  echo "ERROR: curl-impersonate binary (curl_chrome131/124/116) not found. Install curl-impersonate."
  exit 1
fi

SYMBOL="AAPL"
COOKIE_JAR="cookies.txt"
PAGE_OUT="yf_session.html"
OUT_JSON="asset_profile.json"

# === Headers taken from yahooquery/constants.py -> BROWSERS["chrome124"] ===
UA='Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36'
COMMON_HDRS=(
  -H 'sec-ch-ua: "Chromium";v="124", "Google Chrome";v="124", "Not-A.Brand";v="99"'
  -H 'sec-ch-ua-mobile: ?0'
  -H 'sec-ch-ua-platform: "macOS"'
  -H 'upgrade-insecure-requests: 1'
  -H "user-agent: $UA"
  -H 'accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7'
  -H 'sec-fetch-site: none'
  -H 'sec-fetch-mode: navigate'
  -H 'sec-fetch-user: ?1'
  -H 'sec-fetch-dest: document'
  -H 'accept-encoding: gzip, deflate, br, zstd'
  -H 'accept-language: en-US,en;q=0.9'
  -H 'priority: u=0, i'
)

# Some endpoints respond better with explicit HTTP/2 over impersonated clients.
HTTP2=(--http2)

# ==== 1) Initialize session (same flow as session_management.setup_session) ====
"${CURL_BIN}" -sSL --compressed "${HTTP2[@]}" -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
  "${COMMON_HDRS[@]}" \
  'https://finance.yahoo.com' -o "$PAGE_OUT"

# Handle consent redirect (POST) if present
if grep -q 'consent.yahoo.com' "$PAGE_OUT" || grep -q 'collectConsent' "$PAGE_OUT"; then
  CSRF=$(grep -oP 'name="csrfToken"\s+value="\K[^"]+' "$PAGE_OUT" || true)
  SID=$(grep -oP 'name="sessionId"\s+value="\K[^"]+' "$PAGE_OUT" || true)
  if [ -n "${CSRF:-}" ] && [ -n "${SID:-}" ]; then
    "${CURL_BIN}" -sS --compressed "${HTTP2[@]}" -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
      "${COMMON_HDRS[@]}" \
      -X POST 'https://consent.yahoo.com/v2/collectConsent' \
      --data-urlencode 'agree=agree' \
      --data-urlencode 'agree=agree' \
      --data-urlencode 'consentUUID=default' \
      --data-urlencode "sessionId=$SID" \
      --data-urlencode "csrfToken=$CSRF" \
      --data-urlencode 'originalDoneUrl=https://finance.yahoo.com' \
      --data-urlencode 'namespace=yahoo' \
      -o /dev/null
  fi
fi

# ==== 2) Get crumb (session_management.get_crumb) ====
CRUMB=$(
  "${CURL_BIN}" -sS --compressed "${HTTP2[@]}" -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
    "${COMMON_HDRS[@]}" \
    'https://query2.finance.yahoo.com/v1/test/getcrumb'
)

# ==== 3) quoteSummary call for assetProfile (ticker.asset_profile/_quote_summary) ====
# yahooquery merges default params: lang=en-US, region=US, corsDomain=finance.yahoo.com, formatted=false, crumb=...
QS_PARAMS=(
  --get
  --data-urlencode 'formatted=false'
  --data-urlencode 'lang=en-US'
  --data-urlencode 'region=US'
  --data-urlencode 'corsDomain=finance.yahoo.com'
  --data-urlencode 'modules=assetProfile'
)
if [ -n "$CRUMB" ]; then
  QS_PARAMS+=( --data-urlencode "crumb=$CRUMB" )
fi

# Although yahooquery reuses the same headers for all requests, adding Origin/Referer often helps with edge filtering.
API_HDRS=("${COMMON_HDRS[@]}" \
  -H 'origin: https://finance.yahoo.com' \
  -H "referer: https://finance.yahoo.com/quote/${SYMBOL}"
)

"${CURL_BIN}" -sS --compressed "${HTTP2[@]}" -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
  "${API_HDRS[@]}" \
  "https://query2.finance.yahoo.com/v10/finance/quoteSummary/${SYMBOL}" \
  "${QS_PARAMS[@]}" \
  -o "$OUT_JSON"

# Optional: simple retry-on-429 (yahooquery doesn't do this, but it won't change the on-wire shape except repeating the same call)
if jq -e '.quoteSummary.error // .finance.error // empty' "$OUT_JSON" >/dev/null 2>&1 || grep -q 'Too Many Requests' "$OUT_JSON"; then
  echo "429 detected; retrying once after short backoff..." >&2
  sleep 2
  "${CURL_BIN}" -sS --compressed "${HTTP2[@]}" -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
    "${API_HDRS[@]}" \
    "https://query2.finance.yahoo.com/v10/finance/quoteSummary/${SYMBOL}" \
    "${QS_PARAMS[@]}" \
    -o "$OUT_JSON"
fi

echo "Wrote ${OUT_JSON}"
