#!/usr/bin/env bash
set -euo pipefail
script_dir=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
checkout=$(cd -- "$script_dir/.." && pwd -P)
cd -- "$checkout"
profile=debug
args=()
dev=0
for arg in "$@"; do
    case "$arg" in
        --release) [[ "$profile" == debug ]] || exit 2; profile=release; args+=(--release) ;;
        --dev) [[ "$dev" == 0 ]] || exit 2; dev=1; args+=(--features dev) ;;
        *) printf 'Usage: %s [--release] [--dev]\n' "$0" >&2; exit 2 ;;
    esac
done
export CARGO_TARGET_DIR="$checkout/target"
cargo build --locked -p isekai --target wasm32-unknown-unknown "${args[@]}"
bindgen=$("$script_dir/setup-wasm-bindgen.sh")
output="$checkout/target/web-isekai"
mkdir -p -- "$output"
"$bindgen" --target web --out-dir "$output" --out-name isekai "$CARGO_TARGET_DIR/wasm32-unknown-unknown/$profile/isekai.wasm"
cp -- "$checkout/web/isekai/index.html" "$output/index.html"
printf 'Web build: %s\n' "$output"
