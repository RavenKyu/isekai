#!/usr/bin/env bash
# Sourced by the two entry points; resolve this checkout's pin and the shared install.
rn_emsdk_init() {
    local script_dir common_dir
    script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
    RN_CHECKOUT=$(git -C "$script_dir" rev-parse --show-toplevel)
    common_dir=$(git -C "$script_dir" rev-parse --path-format=absolute --git-common-dir)
    RN_PRIMARY_ROOT=$(cd -- "$common_dir/.." && pwd -P)
    RN_EMSDK_VERSION=$(cat -- "$RN_CHECKOUT/.emsdk-version")
    if [[ ! "$RN_EMSDK_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        printf 'Invalid .emsdk-version: %s\n' "$RN_EMSDK_VERSION" >&2
        return 1
    fi
    RN_TOOLS="$RN_PRIMARY_ROOT/.tools"
    RN_EMSDK_DIR="$RN_TOOLS/emsdk"
    RN_EMSDK_MARKER="$RN_EMSDK_DIR/.rn-installed"
}

rn_emsdk_complete() {
    [[ -f "$RN_EMSDK_MARKER" && -f "$RN_EMSDK_DIR/emsdk_env.sh" ]] &&
        [[ "$(cat -- "$RN_EMSDK_MARKER")" == "$RN_EMSDK_VERSION" ]]
}

rn_emsdk_load() {
    local compiler version_output first_line
    # Upstream's environment script is not written for nounset.
    set +u
    # shellcheck disable=SC1091
    EMSDK_QUIET=1 source "$RN_EMSDK_DIR/emsdk_env.sh"
    set -u
    hash -r
    compiler=$(command -v emcc)
    if [[ "$compiler" != "$RN_EMSDK_DIR/"* ]]; then
        printf 'emcc did not resolve inside %s: %s\n' "$RN_EMSDK_DIR" "$compiler" >&2
        return 1
    fi
    version_output=$(emcc --version)
    first_line=${version_output%%$'\n'*}
    if [[ "$first_line" != *" $RN_EMSDK_VERSION "* && "$first_line" != *" $RN_EMSDK_VERSION" ]]; then
        printf 'Expected emcc %s, got: %s\n' "$RN_EMSDK_VERSION" "$first_line" >&2
        return 1
    fi
}
