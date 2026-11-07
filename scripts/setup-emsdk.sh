#!/usr/bin/env bash
set -euo pipefail
script_dir=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
# shellcheck source=scripts/emsdk-common.sh
source "$script_dir/emsdk-common.sh"
rn_emsdk_init
mkdir -p -- "$RN_TOOLS"
exec {emsdk_lock}>"$RN_TOOLS/emsdk.lock"
flock -x "$emsdk_lock"
if rn_emsdk_complete; then
    rn_emsdk_load
    exit 0
fi
# Only this fixed, nonempty child of the primary checkout may be removed.
[[ -n "$RN_PRIMARY_ROOT" && "$RN_EMSDK_DIR" == "$RN_PRIMARY_ROOT/.tools/emsdk" ]]
rm -rf -- "$RN_EMSDK_DIR"
git clone --depth 1 --branch "$RN_EMSDK_VERSION" -- \
    "${RN_EMSDK_REPO:-https://github.com/emscripten-core/emsdk.git}" "$RN_EMSDK_DIR"
(cd -- "$RN_EMSDK_DIR" && ./emsdk install "$RN_EMSDK_VERSION" && ./emsdk activate "$RN_EMSDK_VERSION")
rn_emsdk_load
printf '%s\n' "$RN_EMSDK_VERSION" >"$RN_EMSDK_MARKER"
