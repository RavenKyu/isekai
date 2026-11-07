#!/bin/sh
set -eu

fail() {
    printf 'rn-smoke: fail %s\n' "$*" >&2
    exit 1
}

for tool in cargo xvfb-run Xvfb timeout python3; do
    command -v "$tool" >/dev/null 2>&1 || fail "missing required tool: $tool"
done

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd -- "$script_dir/.."
# Always create our own X display; never connect SDL to the person's Wayland session.
unset WAYLAND_DISPLAY
export SDL_VIDEODRIVER=x11 LIBGL_ALWAYS_SOFTWARE=1
# Reserve two seconds for forced cleanup within the 60-second total budget.
if timeout --kill-after=2s 58s xvfb-run -a cargo test --locked --test smoke_native -- --ignored; then
    exit 0
else
    status=$?
    fail "native smoke failed (status=$status; deadline=60s)"
fi
