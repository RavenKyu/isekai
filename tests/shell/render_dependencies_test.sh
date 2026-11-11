#!/usr/bin/env bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
cargo metadata --locked --no-deps --format-version 1 --manifest-path "$root/Cargo.toml" |
python3 -c '
import json,sys
package=next(p for p in json.load(sys.stdin)["packages"] if p["name"]=="isekai-render")
deps=package["dependencies"]
assert not any(d["name"]=="raw-window-handle" for d in deps), "unused direct window handle dependency"
pollster=[d for d in deps if d["name"]=="pollster"]
assert pollster and all(d["kind"]=="dev" for d in pollster), "pollster must be test-only"
print("render dependency boundary: ok")'
