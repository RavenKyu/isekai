#!/usr/bin/env bash
set -euo pipefail
script_dir=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
checkout=$(cd -- "$script_dir/.." && pwd -P)
fail() { printf 'rn-smoke: fail %s\n' "$*" >&2; exit 1; }
for tool in python3 timeout mktemp; do
    command -v "$tool" >/dev/null 2>&1 || fail "missing required tool: $tool"
done
# Timeout's process group covers Chrome and the HTTP server as well as this shell.
if [[ ${RN_SMOKE_WEB_BOUNDED:-0} != 1 ]]; then
    if RN_SMOKE_WEB_BOUNDED=1 timeout --kill-after=2s 58s "$0"; then
        exit 0
    else
        status=$?
        fail "web smoke failed (status=$status; deadline=60s)"
    fi
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
for file in index.html rusty-nails.js rusty-nails.wasm; do
    [[ -s "$checkout/target/web/$file" ]] || fail "missing web artifact: $file; run scripts/build-web.sh"
done
scratch=$(mktemp -d "$checkout/target/smoke-web.XXXXXX")
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
PORT=0 "$script_dir/serve-web.sh" >"$scratch/server.log" 2>&1 &
server_pid=$!
printf '%s\n' "$server_pid" >"$scratch/server.pid"
url=$(python3 "$script_dir/smoke-web-check.py" ready "$scratch/server.log")
printf 'Web smoke URL: %s; evidence: %s\n' "$url" "$scratch" >&2
python3 "$script_dir/smoke-web-browser.py" "$browser" "${url}?smoke=3" "$scratch" \
    || fail "Chrome failed; evidence: $scratch"
python3 "$script_dir/smoke-web-check.py" check "$scratch"
