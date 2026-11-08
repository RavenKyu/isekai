#!/usr/bin/env bash
set -euo pipefail
script_dir=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
checkout=$(cd -- "$script_dir/.." && pwd -P)
cd -- "$checkout"
profile=debug
args=()
dev=0
usage() { printf 'Usage: %s [--release] [--dev]\n' "$0" >&2; exit 2; }
for arg in "$@"; do
    case "$arg" in
        --release)
            [[ "$profile" == debug ]] || usage
            profile=release
            args+=(--release)
            ;;
        --dev)
            [[ "$dev" == 0 ]] || usage
            dev=1
            args+=(--features dev)
            ;;
        *) usage ;;
    esac
done
# Keep all artifacts in this checkout, even if a caller exports CARGO_TARGET_DIR.
export CARGO_TARGET_DIR="$checkout/target"
"$script_dir/with-emsdk.sh" cargo build --locked --bin rusty-nails \
    --target wasm32-unknown-emscripten "${args[@]}"
output="$CARGO_TARGET_DIR/web"
mkdir -p -- "$output"
cp -- "$checkout/web/index.html" "$output/index.html"
cp -- "$CARGO_TARGET_DIR/wasm32-unknown-emscripten/$profile/rusty-nails.js" "$output/rusty-nails.js"
# rustc keeps the bin's hyphen in JS but uses the crate's underscore in WASM.
cp -- "$CARGO_TARGET_DIR/wasm32-unknown-emscripten/$profile/rusty_nails.wasm" "$output/rusty-nails.wasm"
printf 'Web build: %s\n' "$output"
