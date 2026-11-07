#!/usr/bin/env bash
set -euo pipefail
if (( $# == 0 )); then
    printf 'Usage: %s <command> [args...]\n' "$0" >&2
    exit 2
fi
script_dir=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
# shellcheck source=scripts/emsdk-common.sh
source "$script_dir/emsdk-common.sh"
rn_emsdk_init
mkdir -p -- "$RN_TOOLS"
# Keep the shared lock through exec so a setup cannot replace an SDK in use.
exec {emsdk_lock}>"$RN_TOOLS/emsdk.lock"
flock -s "$emsdk_lock"
if ! rn_emsdk_complete; then
    printf 'emsdk %s is not installed; run %s/setup-emsdk.sh\n' "$RN_EMSDK_VERSION" "$script_dir" >&2
    exit 1
fi
rn_emsdk_load
exec "$@"
