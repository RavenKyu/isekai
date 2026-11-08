#!/usr/bin/env bash
# Check both dependency graphs, then inspect a freshly built non-dev executable.
set -euo pipefail
script_dir=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
cd -- "$script_dir/.."
fail() { printf 'check-no-imgui: fail %s\n' "$*" >&2; exit 1; }
args=()
if (( $# )); then
    if [[ $# != 1 || $1 != --release ]]; then
        printf 'Usage: %s [--release]\n' "$0" >&2
        exit 2
    fi
    args+=(--release)
fi
for tool in cargo nm python3; do
    command -v "$tool" >/dev/null 2>&1 || fail "missing required tool: $tool"
done
for target in host wasm32-unknown-emscripten; do
    target_args=()
    if [[ "$target" != host ]]; then
        target_args+=(--target "$target")
    fi
    graph=$(cargo tree --locked -e normal,build --prefix none "${target_args[@]}") \
        || fail "cannot inspect non-dev graph ($target)"
    # Match package names at line start; the root line carries the checkout path.
    if grep -q '^dear-imgui' <<<"$graph"; then
        fail "imgui dependency in non-dev graph ($target)"
    fi
    graph=$(cargo tree --locked -e normal,build --prefix none "${target_args[@]}" --features dev) \
        || fail "cannot inspect dev graph ($target)"
    if ! grep -q '^dear-imgui-rs v' <<<"$graph"; then
        fail "positive control missing dear-imgui-rs ($target)"
    fi
done
# Cargo's artifact message honors target configuration and CARGO_TARGET_DIR.
artifacts=$(cargo build --locked --bin rusty-nails --message-format=json "${args[@]}") \
    || fail 'non-dev build failed'
executable=$(python3 -c '
import json,sys
paths = [row["executable"] for line in sys.stdin if (row := json.loads(line)).get("reason") == "compiler-artifact"
         and row.get("executable") and row["target"]["name"] == "rusty-nails"]
if len(paths) != 1:
    sys.exit("expected one rusty-nails executable artifact")
print(paths[0])' <<<"$artifacts") || fail 'cannot locate non-dev executable'
symbols=$(nm -C -- "$executable") || fail 'cannot read executable symbols'
[[ -n "$symbols" && "$symbols" == *rusty_nails* ]] || fail 'missing application symbols (stripped executable?)'
if [[ "${symbols,,}" == *imgui* ]]; then
    fail 'imgui symbol in non-dev executable'
fi
printf 'check-no-imgui: ok graphs=host,wasm32-unknown-emscripten symbols=%s\n' "$executable"
