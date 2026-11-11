#!/usr/bin/env bash
set -euo pipefail
script_dir=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
checkout=$(cd -- "$script_dir/.." && pwd -P)
fail() { printf 'isekai-smoke: fail %s\n' "$*" >&2; exit 1; }
if (( $# > 1 )) || { (( $# == 1 )) && [[ "$1" != --dev && "$1" != --play && "$1" != --webgl2 && "$1" != --no-webgpu ]]; }; then
    printf 'Usage: %s [--dev|--play|--webgl2|--no-webgpu]\n' "$0" >&2
    exit 2
fi
for tool in python3 timeout mktemp; do
    command -v "$tool" >/dev/null 2>&1 || fail "missing required tool: $tool"
done
# Preserve the requested mode across the bounded re-exec. A dropped --play or
# --dev must fail before launching a different smoke and accepting its result.
requested_mode=${1:-blank}
# Timeout's process group covers Chrome and the HTTP server as well as this shell.
if [[ ${ISEKAI_SMOKE_WEB_BOUNDED:-0} != 1 ]]; then
    if ISEKAI_SMOKE_WEB_EXPECTED_MODE="$requested_mode" ISEKAI_SMOKE_WEB_BOUNDED=1 timeout --kill-after=2s 58s "$0" "$@"; then
        exit 0
    else
        status=$?
        fail "web smoke failed (status=$status; deadline=60s)"
    fi
fi
if [[ ${ISEKAI_SMOKE_WEB_EXPECTED_MODE:-$requested_mode} != "$requested_mode" ]]; then
    fail 'web smoke mode was lost across bounded re-exec'
fi
if [[ -n ${CHROME:-} ]]; then
    browser=$(command -v "$CHROME") || fail "CHROME not executable: $CHROME"
elif command -v google-chrome >/dev/null 2>&1; then
    browser=$(command -v google-chrome)
elif command -v chromium >/dev/null 2>&1; then
    browser=$(command -v chromium)
else
    fail 'missing browser: set CHROME or install google-chrome/chromium'
fi
for file in index.html isekai.js isekai_bg.wasm; do
    [[ -s "$checkout/target/web-isekai/$file" ]] || fail "missing web artifact: $file; run scripts/build-web-isekai.sh"
done
scratch=$(mktemp -d "$checkout/target/smoke-web-isekai.XXXXXX")
server_pid=
cleanup() {
    if [[ -n "$server_pid" ]]; then
        kill "$server_pid" 2>/dev/null || true
        wait "$server_pid" 2>/dev/null || true
    fi
    # About 40MB of Chrome state per run, and none of it is evidence.
    rm -rf -- "$scratch/chrome-profile" || true
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM
PORT=0 "$script_dir/serve-web-isekai.sh" >"$scratch/server.log" 2>&1 &
server_pid=$!
printf '%s\n' "$server_pid" >"$scratch/server.pid"
url=$(python3 "$script_dir/smoke-web-check.py" ready "$scratch/server.log")
printf 'Web smoke URL: %s; evidence: %s\n' "$url" "$scratch" >&2
# example is the code-only entry in src/sprite/embedded.rs.
query='?smoke=3'
if [[ ${1:-} == --play ]]; then query+='&play=example'; fi
if [[ ${1:-} == --webgl2 ]]; then query+='&backend=webgl2'; fi
no_webgpu=0
if [[ ${1:-} == --no-webgpu ]]; then no_webgpu=1; fi
python3 "$script_dir/smoke-web-browser.py" "$browser" "${url}${query}" "$scratch" isekai-log isekai-smoke: isekai-canvas "$no_webgpu" \
    || fail "Chrome failed; evidence: $scratch"
python3 "$script_dir/smoke-web-check.py" isekai "$scratch" "$requested_mode"
