#!/usr/bin/env bash
# Check both dependency graphs, then inspect a freshly built non-dev executable.
set -euo pipefail
script_dir=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
cd -- "$script_dir/.."
fail() { printf 'check-no-egui: fail %s\n' "$*" >&2; exit 1; }
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
for target in host wasm32-unknown-unknown; do
    target_args=()
    if [[ "$target" != host ]]; then
        target_args+=(--target "$target")
    fi
    graph=$(cargo tree -p isekai --locked -e normal,build --prefix none "${target_args[@]}") \
        || fail "cannot inspect non-dev graph ($target)"
    if grep -Eq '^(egui|egui-wgpu|egui-winit|isekai-editor) v' <<<"$graph"; then
        fail "egui dependency in non-dev graph ($target)"
    fi
    if grep -Eq '^(sdl3|sdl3-sys|dear-imgui[^ ]*) v' <<<"$graph"; then
        fail "old stack dependency ($target non-dev)"
    fi
    graph=$(cargo tree -p isekai --locked -e normal,build --prefix none "${target_args[@]}" --features dev) \
        || fail "cannot inspect dev graph ($target)"
    for package in egui egui-wgpu egui-winit isekai-editor; do
        grep -q "^$package v" <<<"$graph" || fail "positive control missing $package ($target)"
    done
    if grep -Eq '^(sdl3|sdl3-sys|dear-imgui[^ ]*) v' <<<"$graph"; then
        fail "old stack dependency ($target dev)"
    fi
done
# Cargo's artifact message honors target configuration and CARGO_TARGET_DIR.
artifacts=$(cargo build -p isekai --locked --bin isekai --message-format=json "${args[@]}") \
    || fail 'non-dev build failed'
executable=$(python3 -c '
import json,sys
paths = [row["executable"] for line in sys.stdin if (row := json.loads(line)).get("reason") == "compiler-artifact"
         and row.get("executable") and row["target"]["name"] == "isekai"]
if len(paths) != 1:
    sys.exit("expected one isekai executable artifact")
print(paths[0])' <<<"$artifacts") || fail 'cannot locate non-dev executable'
symbols=$(nm -C -- "$executable") || fail 'cannot read executable symbols'
[[ -n "$symbols" && "$symbols" == *isekai* ]] || fail 'missing application symbols (stripped executable?)'
if [[ "$symbols" == *egui::* || "$symbols" == *isekai_editor* ]]; then
    fail 'egui symbol in non-dev executable'
fi
printf 'check-no-egui: ok graphs=host,wasm32-unknown-unknown symbols=%s\n' "$executable"
