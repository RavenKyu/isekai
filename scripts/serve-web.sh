#!/usr/bin/env bash
set -euo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
checkout=$(cd -- "$script_dir/.." && pwd -P)
output="$checkout/target/web"
if [[ ! -f "$output/index.html" ]]; then
    printf 'No web build; run %s/build-web.sh first\n' "$script_dir" >&2
    exit 1
fi
port=${PORT:-8080}
if [[ ! "$port" =~ ^[0-9]+$ ]] || (( ${#port} > 5 )) || (( 10#$port > 65535 )); then
    printf 'Invalid PORT: %s\n' "$port" >&2
    exit 2
fi
if [[ "$port" != 0 ]]; then
    printf 'App URL: http://127.0.0.1:%s/\n' "$port"
fi
# PORT=0 lets the OS allocate a free port atomically; Python prints the actual port.
exec python3 -u -m http.server --bind 127.0.0.1 --directory "$output" "$port"
