# 의존성과 패치 관리

버전은 아래 정본 파일에서 관리한다. 표의 값은 현재 checkout의 값이며
판을 올릴 때는 정본을 먼저 바꾸고 문서와 [네이티브 검사](development.md),
[웹 검사](web.md)를 함께 확인한다. 처음 실행은 [개발 시작](README.md)을 따른다.

## 고정값과 이유

| 항목 | 현재 값과 정본 | 이유 |
| --- | --- | --- |
| Rust | `rust-toolchain.toml`의 channel `1.99.1`, target `wasm32-unknown-emscripten`, rustfmt·clippy | dear-imgui 0.22 계열의 MSRV는 1.95다. Cargo.toml의 rust-version은 최소 버전이며 실행 툴체인은 별도로 고정한다. |
| emsdk | `.emsdk-version`의 `6.0.13` | Rust의 Emscripten 링크 방식과 `sdl3-sys`의 SDL3 소스 빌드를 실제 빌드·실행으로 검증한 조합이다. Emscripten의 SDL3 포트는 쓰지 않는다. 설치·CI에서 이 파일을 읽고 중복 pin을 만들지 않는다. |
| dear-imgui-rs/sdl3/glow | `Cargo.toml`에서 각각 `=0.22.0`, optional이며 `dev` feature에서만 포함 | core·platform·renderer를 같은 계열로 맞추고 비dev 빌드에서는 제외한다. |
| sdl3 | `Cargo.toml`의 `0.20`, `Cargo.lock`의 `0.20.0` | dear-imgui-sdl3가 요구하는 0.20과 맞춰 Window·Event 타입이 갈라지지 않게 한다. |
| sdl3-sys | `Cargo.lock`의 `0.7.3+SDL-3.4.20` | SDL3의 소스 빌드와 FFI 버전을 잠근다. 첫 빌드는 시스템 SDL 대신 이 소스를 컴파일한다. |
| glow | `Cargo.toml`의 `0.18`, `Cargo.lock`의 `0.18.0` | 네이티브 OpenGL과 Emscripten WebGL 2 경로를 같은 renderer에서 사용한다. |
| png | `Cargo.toml`의 `0.18`, 모든 빌드 | 순수 Rust PNG 디코더·인코더여서 Emscripten에서도 쓴다. PNG만 필요하므로 범용 image와 기본 rayon 의존성을 추가하지 않는다. |
| serde | `Cargo.toml`의 `1`, derive 포함, 모든 빌드 | 편집기·런타임이 같은 엄격한 v1 구조체를 읽고 쓴다. |
| serde_json | `Cargo.toml`의 `1`, 모든 빌드 | JSON 사이드카 읽기·정규형 pretty 저장. |
| rfd | `Cargo.toml`의 `0.18`, 네이티브 target optional, `dev`가 활성화 | 네이티브 편집기 파일 대화상자. 비dev와 웹의 두 feature 모드에는 포함하지 않는다. |

Cargo.toml의 범위 제약과 실제 해결된 버전은 다르다. 앱 저장소는 `Cargo.lock`을
추적하고 검사·웹 빌드에서 `--locked`를 사용한다. 다음으로 현재 값을 확인한다.

```sh
cat rust-toolchain.toml .emsdk-version
rustc -vV
scripts/with-emsdk.sh emcc --version
cargo tree --locked -i sdl3
```

`dev`는 default feature에 포함되지 않으며 debug/release 프로필과도 독립적이다.
`glow`와 `sdl3`는 두 모드 모두 사용한다. imgui 세 crate의 버전이나 로컬 패치를
바꾸지 않고 의존성을 optional로 분리했다. 비dev와 dev 모드의 의존성은 다음 명령으로 확인한다.

```sh
cargo tree --locked -e normal,build
cargo tree --locked -e normal,build --features dev -i dear-imgui-rs
cargo tree --locked -e normal,build --target wasm32-unknown-emscripten
cargo tree --locked -e normal,build --target wasm32-unknown-emscripten --features dev -i dear-imgui-rs
cargo tree --locked -e normal,build -i rfd --features dev
scripts/check-no-imgui.sh
```

비dev 그래프에는 dear-imgui와 rfd가 없어야 하고 host dev 그래프에는 둘 다 있어야 한다.
웹 dev 그래프에는 imgui만 있고 rfd는 없어야 한다. `scripts/check-no-imgui.sh`는 이 양성
대조와 새로 빌드한 비dev 실행 파일의 imgui·rfd 기호까지 검사하며, 기호가 지워진
바이너리도 거부한다. `scripts/check-no-imgui.sh --release`는 release 바이너리의 기호 검사다.
`Cargo.lock`은 두 모드에서 공통이다. v0.3.0에서 png·serde·serde_json·rfd와 그 하위 의존성을
추가하며 갱신했고 기존 패키지의 버전은 올리지 않았다. feature를 선택할 때마다 잠금 파일을
다시 만드는 것은 아니다. [스프라이트 안내](sprites.md)에 새 형식과 실행 범위가 있다.

SDK는 [공유 설치 절차](web.md#sdk-설치와-worktree-공유)로 준비한다. Rust std를 만든
emsdk 버전과 이 pin이 일치한다고 보장하는 것은 아니다. LLVM·C++ ABI 호환성은
단순 링크 성공만으로 판정하지 않고 브라우저 렌더링과 스모크까지 확인한다.

## 왜 로컬 패치가 필요한가

upstream dear-imgui 0.22의 공식 wasm 경로와 Emscripten은 같은 경로가 아니다.
공개 crate 그대로는 build-support의 wasm feature gate와 native ABI target 목록이
Emscripten을 거부하고, SDL3 backend의 target별 Rust 의존성에도 Emscripten이 빠져 있다.
프로젝트는 `wasm32-unknown-emscripten`을 유지하기로 결정해 `Cargo.toml`의
`[patch.crates-io]`로 아래 두 복사본을 사용한다. 이 경로는 **upstream 미지원**이며
판을 올릴 때마다 패치를 확인해야 한다. native에도 같은 patch가 적용된다.

| 복사본 | 승인된 변경 |
| --- | --- |
| `third_party/dear-imgui-build-support` | `src/binding/toolchain.rs`에서 정확히 Emscripten target을 허용하고, `src/binding/mod.rs`에 NonWindows ABI target을 추가한다. `src/binding/tests.rs`의 Emscripten 거부 단언과 NonWindows 고정 해시 기대값을 갱신한다. wasip1·wasip2 거부는 유지한다. |
| `third_party/dear-imgui-sdl3` | 정규화된 `Cargo.toml`에 Emscripten의 `sdl3 = "0.20"` 의존성을 추가한다. `Cargo.toml.orig`은 수정하지 않는다. |

원본은 crates.io 0.22.0 `.crate`이며 출처·SHA256과 과거 스파이크 기록은
`third_party/README.md`에 있다. 현재 앱 실행 명령은 [웹 안내](web.md)를 따른다.
스파이크의 임시 `web_spike` 예제는 앱 통합 때 제거했으므로 현재 실행 대상으로
사용하지 않는다. `dear-imgui-sys`와 `dear-imgui-rs`에는 로컬 패치가 없다.
승인된 diff는 추가 19줄·삭제 7줄로 총 26줄이며 최초 스파이크 예산은 100줄이었다.

렌더러는 `dear-imgui-glow`다. 공식 opengl3 backend shim은 wasm에서 제외되므로
`opengl3-renderer` feature를 켜지 않는다. 앱은 네이티브에서 OpenGL 3.3 core,
웹에서 GLES 3.0/WebGL 2 컨텍스트를 만든다.

## 링크 설정을 바꿀 때

유일한 target별 링크 플래그 정본은 `.cargo/config.toml`이다.

| 플래그 | 현재 설정의 목적 |
| --- | --- |
| `-sDEFAULT_TO_CXX` | Rust가 호출하는 emcc 링크에 C++ 런타임을 포함한다. |
| `-sMIN_WEBGL_VERSION=2`, `-sMAX_WEBGL_VERSION=2` | WebGL 2를 사용한다. |
| `-sALLOW_MEMORY_GROWTH=1` | WASM 메모리 증가를 허용한다. |
| `-sSTACK_SIZE=1MB` | 작은 기본 스택 대신 1MB를 지정한다. |
| `-sASSERTIONS=1` | 첫 실행의 ABI·런타임 진단을 켠다. 현재 release 빌드에도 적용된다. |

LTO는 켜지 않는다. Rust와 emcc의 LLVM이 달라 bitcode 호환 문제가 생길 수 있다.
Rust의 `-fwasm-exceptions`와 Emscripten 기본 `WASM_LEGACY_EXCEPTIONS` 조합도
임의로 바꾸지 않는다. 실제 검증은 이 설정 전체의 성공이며, 모든 플래그를 하나씩
제거해 최소 필요성을 증명한 것은 아니다.

## 다음 판 올림 절차

1. 바꿀 Rust·SDK·crate 버전과 패치 범위를 트래커에서 먼저 정한다. 원래 스파이크는
   4시간·100줄 제한이며 sys/core까지 패치하거나 예산·실행 조건을 넘으면 NO-GO로
   멈추고 사람이 다음 경로를 결정했다. 판 올림도 지원 범위를 임의로 넓히지 않는다.
2. 새 원본 `.crate`의 출처와 SHA256을 확인하고 **수정 전 전체 복사본을 별도 커밋**한다.
   그 기준과의 diff로 패치를 검토한다. manifest는 정규화된 `Cargo.toml`을 고친다.
3. Emscripten만 허용하고 WASI 거부를 남긴다. target 정책 해시가 바뀌면 해당 단언
   변경도 검토 범위에 넣는다. 앱의 sdl3와 backend가 같은 타입 버전을 쓰는지 확인한다.
4. build-support의 `binding-spec` feature 전체 테스트를 실행한다. published `.crate`에는
   테스트가 include하는 upstream monorepo 파일이 없으므로 같은 판의 core와 여덟
   extension sys crate fixture를 원래 상대 경로에 먼저 준비해야 한다. fixture 없이
   실행할 때의 파일 누락을 패치 성공이나 테스트 생략으로 처리하지 않는다. 현 판의
   전체 결과는 92개 통과이며, 일반 앱의 Cargo 테스트는 이 검사를 대신하지 않는다.
5. 네이티브 검사·스모크, SDK 가짜 저장소 테스트, 웹 debug/release 빌드·스모크를
   [개발 검사](development.md#평소-검사-순서)와 [웹 안내](web.md)대로 실행한다.
   새 브라우저 스크린샷에 실제 텍스트가 보이고 console/HTTP 오류가 없는지 확인한다.
6. 정본 pin, 잠금 파일, 원본 SHA256, 승인된 patch diff와 문서를 함께 갱신한다.
   실측 Rust/LLVM·emcc 버전, 성공한 설정, 크기와 검사 결과를 트래커에 기록한다.
   [CI 상태](README.md#더-읽기와-ci-상태)의 원격 미실행과 로컬 성공을 구분한다.

Decided in: rn-35g9, rn-7w7b, rn-fcuz
