#!/usr/bin/env bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT
repo="$scratch/repo with spaces"
mkdir -p "$repo/scripts" "$scratch/bin"
git -C "$scratch" init -q "$repo"
cp "$root/scripts/setup-wasm-bindgen.sh" "$repo/scripts/"
cat > "$repo/Cargo.lock" <<'LOCK'
version = 4
[[package]]
name = "wasm-bindgen"
version = "0.2.130"
LOCK
cat > "$scratch/bin/cargo" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail
printf 'install\n' >> "$ISEKAI_TEST_LOG"
while (( $# )); do
    if [[ "$1" == --root ]]; then dest=$2; break; fi
    shift
done
mkdir -p "$dest/bin"
cat > "$dest/bin/wasm-bindgen" <<'TOOL'
#!/usr/bin/env bash
printf 'wasm-bindgen 0.2.130\n'
TOOL
chmod +x "$dest/bin/wasm-bindgen"
FAKE
chmod +x "$scratch/bin/cargo"
export PATH="$scratch/bin:$PATH" ISEKAI_TEST_LOG="$scratch/install.log"
"$repo/scripts/setup-wasm-bindgen.sh" > "$scratch/first" &
first=$!
"$repo/scripts/setup-wasm-bindgen.sh" > "$scratch/second" &
second=$!
wait "$first"; wait "$second"
cmp "$scratch/first" "$scratch/second"
[[ $(wc -l < "$ISEKAI_TEST_LOG") == 1 ]]
install="$repo/.tools/wasm-bindgen/0.2.130"
rm "$install/.complete"
printf 'broken\n' > "$install/bin/wasm-bindgen"
"$repo/scripts/setup-wasm-bindgen.sh" > /dev/null
[[ $(wc -l < "$ISEKAI_TEST_LOG") == 2 ]]
printf '#!/usr/bin/env bash\nprintf "wasm-bindgen 0.0.1\\n"\n' > "$install/bin/wasm-bindgen"
if "$repo/scripts/setup-wasm-bindgen.sh" > /dev/null 2>&1; then
    printf 'mismatched CLI was accepted\n' >&2; exit 1
fi
printf 'wasm-bindgen installer contracts: ok\n'
