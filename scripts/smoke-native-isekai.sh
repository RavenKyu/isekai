#!/usr/bin/env bash
set -euo pipefail
script_dir=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
cd -- "$script_dir/.."
args=()
case "$#:${1:-}" in
  0:) ;;
  1:--dev) args+=(--features dev) ;;
  *) printf 'Usage: %s [--dev]\n' "$0" >&2; exit 2 ;;
esac
for tool in cargo xvfb-run Xvfb timeout python3 xdotool; do
  command -v "$tool" >/dev/null 2>&1 || { printf 'isekai-smoke: fail missing required tool: %s\n' "$tool" >&2; exit 1; }
done
unset WAYLAND_DISPLAY
export WGPU_BACKEND=vulkan VK_LOADER_DRIVERS_SELECT='*lvp*'
timeout --kill-after=2s 58s xvfb-run -a --server-args="-screen 0 1600x1000x24 -ardelay 200 -arinterval 30" bash -c '
  cargo test --locked -p isekai-render -- --ignored &&
  cargo test --locked -p isekai "$@" --bin isekai -- --ignored &&
  cargo test --locked -p isekai "$@" --test smoke -- --ignored --test-threads=1 --nocapture
' bash "${args[@]}"
