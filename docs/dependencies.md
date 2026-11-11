# 의존성과 패치 관리

버전은 아래 정본 파일에서 관리한다. 표의 값은 현재 checkout 기준이며,
버전을 올릴 때는 정본 파일을 먼저 수정한 뒤 이 문서와 [네이티브 검사](development.md),
[웹 검사](web.md)를 함께 확인한다. 처음 환경을 준비할 때는 [개발 시작](README.md)을 따른다.


## isekai 스택의 판 맞춤

워크스페이스 의존성의 버전 범위는 루트 `Cargo.toml`, 실제 해결된 버전은 `Cargo.lock`이 정본이다.
egui 릴리스가 요구하는 `egui-wgpu`·`egui-winit`의 `wgpu`·`winit` 버전을 함께 맞춘다.
현재 잠긴 버전은 `egui`/`egui-wgpu`/`egui-winit` 0.37.0, `wgpu` 30.0.1, `winit` 0.30.13이다.
`wgpu`는 WebGPU와 webgl feature를 함께 켜고 WebGL2 downlevel limits를 기준으로 장치를 요청한다.
색 타깃과 시트는 `Rgba8Unorm`이며 nearest와 straight alpha를 쓴다.
`clap`은 CLI 하위 명령 해석, `web-time`은 네이티브·웹 공통 시간 측정, `wasm-bindgen`과
`wasm-bindgen-futures`는 웹 진입점과 비동기 초기화를 맡는다.
`wasm-bindgen-cli`는 반드시 `Cargo.lock`의 `wasm-bindgen`과 동일한 버전으로 설치한다.

`isekai-editor`는 `isekai` 패키지의 optional `dev` 의존성이다. `egui-winit`은 기본 feature를 끄고
`x11`과 `wayland`만 켠다. 기존 SDL3·dear-imgui 로컬 패치와 emsdk는 `rusty-nails` 유지를 위해
v0.6.0까지 남겨 둔다. 두 스택의 부재 검사는 각각 `check-no-egui.sh`와 `check-no-imgui.sh`가 담당한다.

egui 0.37의 기본 feature는 빈 목록이므로 `default_fonts`를 명시한다. 이에 따라 잠긴
`epaint_default_fonts` 0.37.0이 기본 Latin·기호를 제공한다. 한글은
`assets/fonts/nanum-gothic/NanumGothic-Regular.ttf`(3.020, 2,054,744 bytes)를
`isekai-editor`에서만 컴파일할 때 포함한다. 이 파일은 수정하지 않은 SIL OFL 1.1 자산이며,
[자산 기록](../assets/fonts/nanum-gothic/README.md)에 출처 revision·불변 다운로드 URL·SHA256과
라이선스 전문 경로를 적었다. 두 font family의 fallback으로 사용하며 한글 음절 11,172자를 지원한다.
게임 텍스트 렌더링은 이 셸 폰트와 별개다. 기본·한글 폰트 모두 비dev native/wasm에는 포함하지 않는다.


## 고정값과 이유

| 항목 | 현재 값과 정본 | 이유 |
| --- | --- | --- |
| Rust | `rust-toolchain.toml`의 channel `1.99.1`, target `wasm32-unknown-emscripten`·`wasm32-unknown-unknown`, rustfmt·clippy | dear-imgui 0.22 계열의 MSRV는 1.95다. Cargo.toml의 rust-version은 최소 버전이며 실행 툴체인은 두 웹 target과 함께 별도로 고정한다. |
| emsdk | `.emsdk-version`의 `6.0.13` | Rust의 Emscripten 링크 방식과 `sdl3-sys`의 SDL3 소스 빌드를 실제 빌드·실행으로 검증한 조합이다. Emscripten의 SDL3 포트는 쓰지 않는다. 설치·CI에서 이 파일을 읽고 중복 pin을 만들지 않는다. |
| dear-imgui-rs/sdl3/glow | `Cargo.toml`에서 각각 `=0.22.0`, optional이며 `dev` feature에서만 포함 | core·platform·renderer를 같은 계열로 맞추고 비dev 빌드에서는 제외한다. |
| sdl3 | `Cargo.toml`의 `0.20`, `Cargo.lock`의 `0.20.0` | dear-imgui-sdl3가 요구하는 0.20과 맞춰 Window·Event 타입이 갈라지지 않게 한다. |
| sdl3-sys | `Cargo.lock`의 `0.7.3+SDL-3.4.20` | SDL3의 소스 빌드와 FFI 버전을 잠근다. 첫 빌드는 시스템 SDL 대신 이 소스를 컴파일한다. |
| glow | `Cargo.toml`의 `0.18`, `Cargo.lock`의 `0.18.0` | 네이티브 OpenGL과 Emscripten WebGL 2 경로를 같은 renderer에서 사용한다. |
| png | `Cargo.toml`의 `0.18`, 모든 빌드 | 순수 Rust PNG 디코더·인코더여서 Emscripten에서도 쓴다. PNG만 필요하므로 범용 image와 기본 rayon 의존성을 추가하지 않는다. |
| serde | `Cargo.toml`의 `1`, derive 포함, 모든 빌드 | 편집기·런타임이 같은 엄격한 v1 구조체를 읽고 쓴다. |
| serde_json | `Cargo.toml`의 `1`, 모든 빌드 | JSON 사이드카 읽기·정규형 pretty 저장. |
| rfd | `Cargo.toml`의 `0.18`, 네이티브 target optional, `dev`가 활성화 | 네이티브 편집기 파일 대화상자. 비dev와 웹의 두 feature 모드에는 포함하지 않는다. |

`Cargo.toml`에 적힌 범위 제약과 실제 잠긴 버전은 다를 수 있으므로, 앱 저장소는 `Cargo.lock`을
추적하고 검사·웹 빌드에서 `--locked`를 사용한다. 현재 값은 다음 명령으로 확인한다.

```sh
cat rust-toolchain.toml .emsdk-version
rustc -vV
scripts/with-emsdk.sh emcc --version
cargo tree --locked -i sdl3
```

`dev`는 default feature에 포함되지 않으며 debug/release 빌드 프로필과도 독립적이다.
`glow`와 `sdl3`는 두 모드 모두에서 사용하며, imgui 관련 세 crate는 버전이나 로컬 패치를
바꾸지 않은 채 optional 의존성으로 분리했다. 비dev와 dev 모드의 의존성 트리는 다음 명령으로 확인한다.

```sh
cargo tree --locked -e normal,build
cargo tree --locked -e normal,build --features dev -i dear-imgui-rs
cargo tree --locked -e normal,build --target wasm32-unknown-emscripten
cargo tree --locked -e normal,build --target wasm32-unknown-emscripten --features dev -i dear-imgui-rs
cargo tree --locked -e normal,build -i rfd --features dev
scripts/check-no-imgui.sh
```

비dev 의존성 그래프에는 dear-imgui와 rfd가 없어야 하고, host dev 그래프에는 둘 다 있어야 한다.
웹 dev 그래프에는 imgui만 있고 rfd는 없어야 한다. `scripts/check-no-imgui.sh`는 이 양성
대조와 함께 새로 빌드한 비dev 실행 파일에 imgui·rfd 기호가 없는지 검사하며, 기호가 지워진(strip)
바이너리도 거부한다. `scripts/check-no-imgui.sh --release`를 실행하면 release 바이너리의 기호도 검사한다.
`Cargo.lock`은 두 모드가 공유한다. v0.3.0에서 `png`·`serde`·`serde_json`·`rfd`와 하위 의존성을
추가할 때 잠금 파일을 갱신했으며 기존 패키지 버전은 올리지 않았다. feature를 바꿔 빌드할 때마다 잠금 파일을
새로 만들지는 않는다. 새 스프라이트 형식과 실행 범위는 [스프라이트 안내](sprites.md)에서 다룬다.

emsdk는 [공유 설치 절차](web.md#sdk-설치와-worktree-공유)에 따라 준비한다. Rust 표준 라이브러리를 빌드한
emsdk 버전과 저장소의 pin 버전이 완전히 같다고 보장하지는 않으므로, LLVM·C++ ABI 호환성은
단순 링크 성공만으로 판단하지 않고 실제 브라우저 렌더링과 스모크까지 확인한다.

## 왜 로컬 패치가 필요한가

upstream dear-imgui 0.22의 공식 wasm 경로는 Emscripten 경로와 다르다.
배포된 crate를 그대로 쓰면 `build-support`의 wasm feature gate와 네이티브 ABI target 목록이
Emscripten을 거부하며, SDL3 백엔드의 target별 Rust 의존성에도 Emscripten이 빠져 있다.
이 프로젝트는 `wasm32-unknown-emscripten`을 유지하기로 결정했으므로 루트 `Cargo.toml`의
`[patch.crates-io]`를 통해 아래 두 로컬 복사본을 사용한다. 이 구성은 **upstream 미지원 경로**이므로
버전을 올릴 때마다 패치를 확인해야 한다. 네이티브 빌드에도 동일한 패치가 적용된다.

| 복사본 | 승인된 변경 |
| --- | --- |
| `third_party/dear-imgui-build-support` | `src/binding/toolchain.rs`에서 정확히 Emscripten target을 허용하고, `src/binding/mod.rs`에 NonWindows ABI target을 추가한다. `src/binding/tests.rs`의 Emscripten 거부 단언과 NonWindows 고정 해시 기대값을 갱신한다. wasip1·wasip2 거부는 유지한다. |
| `third_party/dear-imgui-sdl3` | 정규화된 `Cargo.toml`에 Emscripten의 `sdl3 = "0.20"` 의존성을 추가한다. `Cargo.toml.orig`은 수정하지 않는다. |

패치 원본은 crates.io에 배포된 0.22.0 `.crate` 아카이브이며, 출처·SHA256 해시·초기 스파이크 기록은
`third_party/README.md`에 정리되어 있다. 현재 웹 앱 실행 방법은 [웹 안내](web.md)를 따른다.
초기 스파이크에서 썼던 임시 `web_spike` 예제는 본 앱에 통합하면서 제거했으므로 더 이상
사용하지 않는다. `dear-imgui-sys`와 `dear-imgui-rs`에는 로컬 패치가 없다.
승인된 전체 diff는 추가 19줄·삭제 7줄(합계 26줄)이며, 최초 스파이크의 패치 예산 상한은 100줄이었다.

렌더러는 `dear-imgui-glow`를 사용한다. 공식 opengl3 백엔드 shim은 wasm 빌드에서 제외되므로
`opengl3-renderer` feature는 켜지 않는다. 앱은 네이티브에서 OpenGL 3.3 core,
웹에서 GLES 3.0/WebGL 2 컨텍스트를 생성한다.

## 링크 설정을 바꿀 때

target별 링크 플래그의 유일한 정본은 `.cargo/config.toml`이다.

| 플래그 | 현재 설정의 목적 |
| --- | --- |
| `-sDEFAULT_TO_CXX` | Rust가 호출하는 emcc 링크에 C++ 런타임을 포함한다. |
| `-sMIN_WEBGL_VERSION=2`, `-sMAX_WEBGL_VERSION=2` | WebGL 2를 사용한다. |
| `-sALLOW_MEMORY_GROWTH=1` | WASM 메모리 증가를 허용한다. |
| `-sSTACK_SIZE=1MB` | 작은 기본 스택 대신 1MB를 지정한다. |
| `-sASSERTIONS=1` | 첫 실행의 ABI·런타임 진단을 켠다. 현재 release 빌드에도 적용된다. |

Rust 컴파일러와 emcc의 LLVM 버전 차이로 bitcode 호환성 문제가 생길 수 있으므로 LTO는 켜지 않는다.
Rust의 `-fwasm-exceptions`와 Emscripten 기본 `WASM_LEGACY_EXCEPTIONS` 조합 역시
임의로 바꾸지 않는다. 현재 설정은 전체 조합을 기준으로 동작을 검증한 것이며, 개별 플래그를 하나씩
빼면서 최소 조건을 증명한 상태는 아니다.

## 다음 판 올림 절차

1. 변경할 Rust·SDK·crate 버전과 패치 허용 범위를 트래커에서 먼저 확정한다. 초기 스파이크는
   4시간·100줄 제한을 두었고, `sys`/`core`까지 패치해야 하거나 예산·실행 조건을 넘기면 NO-GO로
   중단하고 사람이 다음 방향을 결정했다. 버전을 올릴 때도 지원 범위를 임의로 넓히지 않는다.
2. 새 원본 `.crate`의 출처와 SHA256을 확인한 뒤 **수정하지 않은 원본 복사본 전체를 먼저 별도 커밋**한다.
   이후 패치는 그 커밋과의 diff로 검토한다. manifest는 정규화된 `Cargo.toml`을 고친다.
3. Emscripten만 허용하고 WASI 거부는 그대로 유지한다. target 정책 해시가 바뀌면 해당 테스트 단언
   변경도 리뷰 범위에 포함한다. 앱이 쓰는 `sdl3`와 백엔드가 요구하는 `sdl3`의 타입 버전이 일치하는지 확인한다.
4. `build-support`의 `binding-spec` feature 전체 테스트를 실행한다. crates.io에 배포된 `.crate`에는
   테스트가 참조하는 upstream 모노레포 파일들이 빠져 있으므로, 같은 버전의 `core`와 8개
   extension `sys` crate fixture를 원래 상대 경로에 먼저 배치해야 한다. fixture 없이
   실행해 발생한 파일 누락 오류를 패치 성공이나 테스트 생략 사유로 처리하지 않는다. 현재 버전의
   전체 테스트 결과는 92개 통과이며, 일반 앱의 `cargo test`가 이 검사를 대신하지는 않는다.
5. 네이티브 검사·스모크, SDK 가짜 저장소 셸 테스트, 웹 debug/release 빌드·스모크를
   [개발 검사](development.md#평소-검사-순서)와 [웹 안내](web.md) 절차대로 모두 실행한다.
   새 브라우저 스크린샷에 실제 텍스트가 보이고 console/HTTP 오류가 없는지 확인한다.
6. 정본 pin 파일, `Cargo.lock`, 원본 SHA256, 승인된 패치 diff, 관련 문서를 함께 갱신한다.
   실측한 Rust/LLVM·emcc 버전, 성공한 플래그 설정, 크기와 검사 결과를 트래커에 기록한다.
   [CI 상태](README.md#더-읽기와-ci-상태)에 적힌 원격 CI 미실행과 로컬 성공을 구분한다.

Decided in: isk-kznl, isk-srq5, isk-35g9, isk-7w7b, isk-fcuz
