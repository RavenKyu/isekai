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
    if grep -Eq '^(egui|egui-wgpu|egui-winit|epaint|epaint_default_fonts|isekai-editor) v' <<<"$graph"; then
        fail "egui dependency in non-dev graph ($target)"
    fi
    if grep -Eq '^(sdl3|sdl3-sys|dear-imgui[^ ]*) v' <<<"$graph"; then
        fail "old stack dependency ($target non-dev)"
    fi
    graph=$(cargo tree -p isekai --locked -e normal,build --prefix none "${target_args[@]}" --features dev) \
        || fail "cannot inspect dev graph ($target)"
    for package in egui egui-wgpu egui-winit epaint_default_fonts isekai-editor; do
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
# Build the non-dev wasm too: a host symbol check cannot prove its asset boundary.
artifacts=$(cargo build -p isekai --locked --bin isekai --target wasm32-unknown-unknown --message-format=json "${args[@]}") \
    || fail 'non-dev wasm build failed'
wasm=$(python3 -c '
import json,sys
paths = [row["executable"] for line in sys.stdin if (row := json.loads(line)).get("reason") == "compiler-artifact"
         and row.get("executable") and row["target"]["name"] == "isekai"]
if len(paths) != 1:
    sys.exit("expected one isekai wasm executable artifact")
print(paths[0])' <<<"$artifacts") || fail 'cannot locate non-dev wasm'
python3 - "$executable" "$wasm" <<'PY'
from pathlib import Path
import sys

def leb128(data, at):
    value = shift = 0
    while True:
        byte = data[at]
        at += 1
        value |= (byte & 0x7F) << shift
        shift += 7
        if byte < 0x80:
            return value, at

# The wasm counterpart of the nm check: its function names, from the custom
# section called "name".
wasm = Path(sys.argv[2]).read_bytes()
if wasm[:4] != b'\0asm':
    sys.exit(f'check-no-egui: fail not a wasm module: {sys.argv[2]}')
names, at = [], 8
while at < len(wasm):
    size, start = leb128(wasm, at + 1)
    if wasm[at] == 0:
        length, title = leb128(wasm, start)
        if wasm[title:title + length] == b'name':
            names.append(wasm[title + length:start + size])
    at = start + size
if len(names) != 1 or b'isekai' not in names[0]:
    sys.exit('check-no-egui: fail missing wasm function names (stripped wasm?)')
if b'egui' in names[0]:
    sys.exit('check-no-egui: fail egui name in non-dev wasm')

fonts = [path for path in Path('assets/fonts').rglob('*') if path.suffix in ('.ttf', '.otf')]
if not fonts:
    sys.exit('check-no-egui: fail no font assets to check')
for artifact in sys.argv[1:]:
    data = Path(artifact).read_bytes()
    for font in fonts:
        if font.read_bytes() in data:
            sys.exit(f'check-no-egui: fail embedded font {font} in {artifact}')
PY
printf 'check-no-egui: ok graphs=host,wasm32-unknown-unknown symbols=%s,%s fonts=absent(native,wasm)\n' "$executable" "$wasm"
