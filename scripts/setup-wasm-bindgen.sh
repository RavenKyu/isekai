#!/usr/bin/env bash
set -euo pipefail
script_dir=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
checkout=$(git -C "$script_dir" rev-parse --show-toplevel)
common=$(git -C "$checkout" rev-parse --path-format=absolute --git-common-dir)
primary=$(cd -- "$common/.." && pwd -P)
version=$(python3 - "$checkout/Cargo.lock" <<'PY'
import sys,tomllib
with open(sys.argv[1],'rb') as file:
    versions={p['version'] for p in tomllib.load(file)['package'] if p['name']=='wasm-bindgen'}
if len(versions)!=1:
    sys.exit('expected exactly one wasm-bindgen version in Cargo.lock')
print(versions.pop())
PY
)
install="$primary/.tools/wasm-bindgen/$version"
mkdir -p -- "$primary/.tools/wasm-bindgen"
exec 9>"$primary/.tools/wasm-bindgen/.install.lock"
flock 9
if [[ ! -f "$install/.complete" || ! -x "$install/bin/wasm-bindgen" ]]; then
    rm -rf -- "$install"
    cargo install wasm-bindgen-cli --version "=$version" --locked --root "$install" >&2
    [[ $("$install/bin/wasm-bindgen" --version) == "wasm-bindgen $version" ]] || { printf 'wasm-bindgen version mismatch\n' >&2; exit 1; }
    printf '%s\n' "$version" > "$install/.complete"
fi
[[ $(cat "$install/.complete") == "$version" && $("$install/bin/wasm-bindgen" --version) == "wasm-bindgen $version" ]] || { printf 'wasm-bindgen version mismatch\n' >&2; exit 1; }
printf '%s\n' "$install/bin/wasm-bindgen"
