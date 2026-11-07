# Dear ImGui Emscripten spike

These are complete copies of the crates.io 0.22.0 archives. Commit `c931ee4`
preserves their unmodified contents. The route is unsupported upstream.

| Crate | Archive URL | SHA256 |
| --- | --- | --- |
| dear-imgui-build-support | https://static.crates.io/crates/dear-imgui-build-support/dear-imgui-build-support-0.22.0.crate | `5d2683d46e026a93eceeb815ca0f5709b21e089bd99029adc2e52eb1029c5ecb` |
| dear-imgui-sdl3 | https://static.crates.io/crates/dear-imgui-sdl3/dear-imgui-sdl3-0.22.0.crate | `d1758298b1c988e37f90d65da4fcbdfba63ec91f380278daa33512c893f1d7e1` |

Both archives record upstream commit `7424350518efb54693b8acd4a417b69d9d0f015b`
in `.cargo_vcs_info.json`. Preserve their package metadata and license declarations.

The proposed changes are confined to:

- `dear-imgui-build-support/src/binding/toolchain.rs`: admit exactly
  `wasm32-unknown-emscripten` through the feature gate, using the native C++ route.
  The separate wasm provider route remains unchanged.
- `dear-imgui-build-support/src/binding/mod.rs`: add that target to the
  NonWindows ABI profile with OS `emscripten`, empty env/ABI, wasm32, little
  endian, and 32-bit pointers.
- `dear-imgui-build-support/src/binding/tests.rs`: accept Emscripten in the
  feature-contract test while retaining rejection of wasip1 and wasip2. Update
  the NonWindows profile hash assertion at line 215 to
  `fnv1a64:c8c41d87aacca474`, matching the added supported target. This one
  additional assertion was explicitly approved by the person on 2026-11-08.
- `dear-imgui-sdl3/Cargo.toml`: add the missing Emscripten `sdl3 = "0.20"`
  dependency to the normalized manifest. `Cargo.toml.orig` stays untouched.

`git diff --numstat c931ee4 -- third_party/dear-imgui-build-support
third_party/dear-imgui-sdl3` reports 19 additions and 7 deletions: 26 changed
lines against the 100-line budget. Neither dear-imgui-sys nor dear-imgui-rs is
patched. For an upgrade, verify the new archive checksums, preserve a pristine
vendor commit, reapply only an approved patch, and repeat native, target-contract,
Emscripten build, and browser screenshot checks. A successful build alone does
not prove the ABI works.

## Reproducing the investigation

```sh
scripts/setup-emsdk.sh
cargo check --locked
cargo test --locked
scripts/with-emsdk.sh cargo build --locked --example web_spike --target wasm32-unknown-emscripten
mkdir -p target/web
cp web/index.html target/web/
cp target/wasm32-unknown-emscripten/debug/examples/web_spike.{js,wasm} target/web/
python3 -m http.server 18769 --bind 127.0.0.1 --directory target/web
```

Open `http://127.0.0.1:18769/index.html`. The screenshot investigation used Chrome
headless, SwiftShader, 1280x720, and a 20000 ms virtual-time budget. The served
directory contains only staged web artifacts. The spike example owns its state
on the heap and uses `emscripten_set_main_loop_arg`; it is replaced during app
integration. All link flags live in `.cargo/config.toml`; assertions are enabled
for this debug investigation and LTO is not enabled. The HTML maps requested
WASM names to the staged `web_spike.wasm` explicitly.

The published build-support unit tests require files outside their archive from
the upstream monorepo. A bare invocation fails at `include_str!`. For the
investigation, unchanged 0.22.0 archives of dear-imgui-sys and the eight extension
sys crates in `maintained_sources.json` were extracted under
`target/lane-b/test-fixtures/`. Temporary `dear-imgui-sys` and `extensions`
symlinks at this checkout's root supplied the expected monorepo paths. Those
links were removed after running:

```sh
CARGO_TARGET_DIR="$PWD/target/vendor-support" cargo test --locked \
  --manifest-path third_party/dear-imgui-build-support/Cargo.toml --features binding-spec
```

Adding a supported target changes the NonWindows profile hash from
`fnv1a64:6c40e31dc302e556` to `fnv1a64:c8c41d87aacca474`. The initial investigation
stopped with NO-GO because updating that expected value exceeded the original
patch scope. The person explicitly approved this one additional assertion on
2026-11-08; no other patch scope changed. With that correction and the fixtures above, the full suite passes: 92 tests,
0 failures, 0 ignored, 0 filtered; doctests also pass (0 tests). The existing
wasip1/wasip2 rejection assertions remain intact. Together with the successful
Emscripten build and browser screenshot, this satisfies the spike GO criteria.
