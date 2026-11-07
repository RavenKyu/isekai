#!/usr/bin/env bash
set -euo pipefail
settings_before=$(sha256sum "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile" 2>/dev/null || true)
source_root=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)
scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT
fixture="$scratch/primary checkout"
fake_repo="$scratch/fake emsdk"
export RN_TEST_EVENTS="$scratch/events"
mkdir -p "$fixture/scripts" "$fake_repo" "$scratch/fake-bin"
cp -- "$source_root"/scripts/{emsdk-common,setup-emsdk,with-emsdk}.sh "$fixture/scripts/"
printf '6.0.13\n' >"$fixture/.emsdk-version"
printf '/.tools/\n' >"$fixture/.gitignore"
git -C "$fixture" init -q
git -C "$fixture" add .
git -C "$fixture" -c user.name=Test -c user.email=test@example.invalid commit -qm fixture

cat >"$fake_repo/emsdk" <<'FAKE_SDK'
#!/usr/bin/env bash
set -euo pipefail
# Exactly <command> <version>: `activate --permanent` would edit shell start files.
(( $# == 2 )) || exit 2
case "$1" in
    install)
        printf 'install %s\n' "$2" >>"$RN_TEST_EVENTS"
        touch install.partial
        if [[ ${RN_TEST_INTERRUPT:-0} == 1 ]]; then
            kill -TERM "$PPID"
            exit 143
        fi
        sleep 0.3
        mkdir -p upstream/emscripten
        printf '#!/usr/bin/env bash\nprintf "emcc (fixture) %s\\n"\n' "${RN_TEST_COMPILER_VERSION:-$2}" >upstream/emscripten/emcc
        chmod +x upstream/emscripten/emcc
        rm install.partial
        ;;
    activate)
        [[ -x upstream/emscripten/emcc && ! -f install.partial ]]
        printf 'activate %s\n' "$2" >>"$RN_TEST_EVENTS"
        ;;
    *) exit 2 ;;
esac
FAKE_SDK
cat >"$fake_repo/emsdk_env.sh" <<'FAKE_ENV'
[[ -n ${EMSDK_QUIET:-} ]] || echo 'Setting up EMSDK environment'
export PATH="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)/upstream/emscripten:$PATH"
FAKE_ENV
chmod +x "$fake_repo/emsdk"
git -C "$fake_repo" init -q
git -C "$fake_repo" add .
git -C "$fake_repo" -c user.name=Test -c user.email=test@example.invalid commit -qm fake
git -C "$fake_repo" tag 6.0.13
git -C "$fake_repo" tag 6.0.14
export RN_EMSDK_REPO="file://$fake_repo"
git -C "$fixture" worktree add -qb test-lane "$scratch/linked checkout"
lane="$scratch/linked checkout"
setup="$lane/scripts/setup-emsdk.sh"
with="$lane/scripts/with-emsdk.sh"
install_dir="$fixture/.tools/emsdk"

expect_fail() {
    if "$@" >"$scratch/expected-failure.log" 2>&1; then
        printf 'Expected failure: %s\n' "$*" >&2
        exit 1
    fi
}
installs() { awk '/^install / { n++ } END { print n+0 }' "$RN_TEST_EVENTS"; }

# A PATH compiler must never stand in for a missing or mismatched SDK.
printf '#!/usr/bin/env bash\nprintf "emcc (wrong) 0.0.0\\n"\n' >"$scratch/fake-bin/emcc"
chmod +x "$scratch/fake-bin/emcc"
export PATH="$scratch/fake-bin:$PATH"
expect_fail "$with" emcc --version

# Two setups and a command starting during installation share one completed SDK.
"$setup" >"$scratch/setup-1.log" 2>&1 &
first_pid=$!
"$setup" >"$scratch/setup-2.log" 2>&1 &
second_pid=$!
for ((i=0; i<200; i++)); do
    [[ -f "$install_dir/install.partial" ]] && break
    sleep 0.01
done
[[ -f "$install_dir/install.partial" ]]
"$with" emcc --version >"$scratch/compiler.log"
wait "$first_pid"
wait "$second_pid"
[[ $(installs) == 1 && $(cat "$install_dir/.rn-installed") == 6.0.13 ]]
[[ ! -e "$lane/.tools" ]]
[[ $(cat "$scratch/compiler.log") == 'emcc (fixture) 6.0.13' ]]
# shellcheck disable=SC2016
[[ $("$with" printf '<%s>\n' 'two words' 'literal $()') == $'<two words>\n<literal $()>' ]]
expect_fail "$with"
# An exported CDPATH must not redirect or pollute a script's own location.
(cd -- "$lane" && CDPATH=. scripts/with-emsdk.sh true)
"$setup" >"$scratch/repeated.log" 2>&1
[[ $(installs) == 1 ]]

# Kill the installation's parent before activation: no marker, next setup reclones.
rm "$install_dir/.rn-installed"
expect_fail env RN_TEST_INTERRUPT=1 "$setup"
[[ ! -f "$install_dir/.rn-installed" && -f "$install_dir/install.partial" ]]
expect_fail "$with" true
"$setup" >"$scratch/recovery.log" 2>&1
[[ $(installs) == 3 && ! -e "$install_dir/install.partial" ]]

# This worktree's pin controls the shared SDK; the primary pin remains unchanged.
printf '6.0.14\n' >"$lane/.emsdk-version"
expect_fail "$with" true
"$setup" >"$scratch/upgrade.log" 2>&1
[[ $(installs) == 4 && $("$with" emcc --version) == 'emcc (fixture) 6.0.14' ]]
[[ $(cat "$fixture/.emsdk-version") == 6.0.13 ]]

# A running command retains a shared lock until it exits, blocking replacement.
# shellcheck disable=SC2016
"$with" bash -c 'touch "$1"; while [[ ! -f "$2" ]]; do sleep 0.02; done' \
    _ "$scratch/running" "$scratch/release" &
command_pid=$!
for ((i=0; i<200; i++)); do
    [[ -f "$scratch/running" ]] && break
    sleep 0.01
done
[[ -f "$scratch/running" ]]
"$fixture/scripts/setup-emsdk.sh" >"$scratch/locked.log" 2>&1 &
setup_pid=$!
sleep 0.1
[[ $(installs) == 4 ]]
touch "$scratch/release"
wait "$command_pid"
wait "$setup_pid"
[[ $(installs) == 5 ]]

# A matching marker cannot hide the wrong compiler version.
printf '#!/usr/bin/env bash\nprintf "emcc (wrong) 0.0.0\\n"\n' >"$install_dir/upstream/emscripten/emcc"
expect_fail "$fixture/scripts/with-emsdk.sh" true
expect_fail "$fixture/scripts/setup-emsdk.sh"

# A PATH compiler reporting the pinned version cannot stand in for the SDK's own.
printf '#!/usr/bin/env bash\nprintf "emcc (path) 6.0.13\\n"\n' >"$scratch/fake-bin/emcc"
rm "$install_dir/upstream/emscripten/emcc"
expect_fail "$fixture/scripts/with-emsdk.sh" true

# A fresh install that fails verification must not be marked complete.
rm "$install_dir/.rn-installed"
expect_fail env RN_TEST_COMPILER_VERSION=0.0.0 "$fixture/scripts/setup-emsdk.sh"
[[ ! -f "$install_dir/.rn-installed" ]]
[[ -z $(git -C "$fixture" status --short) ]]
[[ "$settings_before" == "$(sha256sum "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile" 2>/dev/null || true)" ]]
printf 'emsdk shell tests: ok (locking, worktrees, spaces, interruption, version, argv, location)\n'
