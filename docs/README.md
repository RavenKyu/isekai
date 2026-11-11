# Rusty Nails 개발 시작

Rust와 SDL3로 네이티브 창과 WebGL 2 앱을 만든다. Dear ImGui 개발 도구는
Cargo의 `dev` feature를 켰을 때만 포함된다. 기본 개발 창은 두 플랫폼에서 공유하며,
스프라이트 편집·추출은 네이티브 dev에서 제공한다.
이 문서는 Linux에서 개발 환경을 준비하고 두 실행 경로를 확인하는 방법을 안내한다.
모든 명령은 전달받은 저장소 checkout 최상단에서 실행한다.


## v0.5.0: isekai 런타임과 기존 편집기

새 게임 런타임 `isekai`는 wgpu와 winit을 사용한다. 네이티브에서는 다음 명령으로 실행한다.

```sh
cargo run -p isekai
cargo run -p isekai -- play tests/fixtures/example.sprites.json cycle
cargo run -p isekai --features dev
```

빈 창은 1280×720 크기이며 제목은 `isekai`다. 창을 닫으면 정상 종료한다. dev 도구는 숨김 상태로
시작하며, 물리 백틱 키로 `View` 메뉴와 `Frame stats`를 여닫는다. `egui Inspection`과
`egui Settings`도 `View` 메뉴에서 연다. 새 셸에는 `Assets` 메뉴가 없으므로,
시트 편집·Regions·Extract는 v0.6.0까지 기존 `cargo run --features dev`를 사용한다.
루트 default member가 여전히 `rusty-nails`이므로 패키지를 생략한 기존 명령도 그대로 동작한다.

새 네이티브 스모크에는 `xdotool`, `mesa-vulkan-drivers`, `libvulkan1`,
`libxkbcommon-x11-dev`가 추가로 필요하다. 새 웹 경로는 다음과 같이 실행한다.

```sh
scripts/build-web-isekai.sh
scripts/serve-web-isekai.sh
```

브라우저에서 `http://127.0.0.1:8080/?play=example`을 연다. 자세한 설치와 검사 방법은
[네이티브 개발](development.md#isekai-워크스페이스와-검사)과
[새 웹 경로](web.md#isekai의-wasm-bindgen-웹-경로)를 참고한다.
CI는 기존 native·web 외에 isekai-native·isekai-web도 함께 검사한다.


## 필요한 것

검증 환경은 Ubuntu 26.04 계열이며, Windows와 macOS 설치 절차는 다루지 않는다.

| 용도 | 필요한 도구와 환경 |
| --- | --- |
| 코드와 Rust | Git, rustup. Rust 채널·target·컴포넌트는 `rust-toolchain.toml`이 정한다. |
| 소스 빌드 | C/C++ 컴파일러, CMake, Make 또는 Ninja, pkg-config. Ubuntu 패키지 이름은 `build-essential`, `cmake`, `ninja-build`, `pkg-config`다. |
| Linux 창·GL | X11 또는 Wayland 개발 라이브러리와 OpenGL 드라이버. 아래 X11 패키지와 Mesa 경로로 네이티브 스모크를 검증했다. |
| 스크립트 | Bash, Python 3, `flock`(util-linux), GNU `timeout`(coreutils). |
| 네이티브 스모크 | `Xvfb`, `xvfb-run`, `xauth`, Mesa software GL. Ubuntu에서는 `xvfb`, `xauth`, `libgl1-mesa-dri`를 준비한다. |
| 웹·웹 스모크 | WebGL 2를 쓸 수 있는 브라우저. 자동 검사는 `google-chrome` 또는 `chromium`의 headless 모드를 사용한다. |
| imgui 부재 검사 | `nm`(binutils). 의존성 그래프와 비dev 실행 파일 기호를 검사한다. |
| 셸 검사 | ShellCheck. Python 보조 도구는 표준 라이브러리만 쓰므로 pip/npm 설치는 없다. |

X11 개발 패키지는 `libx11-dev`, `libxext-dev`, `libxrandr-dev`, `libxcursor-dev`,
`libxi-dev`, `libxfixes-dev`, `libxkbcommon-dev`이며, GL 헤더는 `libegl-dev`,
`libgl-dev`로 설치한다. Wayland를 쓰려면 해당 개발 패키지도 필요하다.
파일 대화상자 crate인 `rfd`가 `wayland-sys`를 빌드할 때 pkg-config로 `wayland-client`를 찾으므로,
`dev` feature 빌드는 X11만 쓰더라도 `libwayland-dev`가 있어야 한다.
SDL3의 다른 선택 기능이 요구하는 라이브러리는 CMake 로그에서 확인한다.
Cargo가 SDL3를 소스에서 빌드하므로 시스템 SDL3 패키지를 미리 설치할 필요는 없다.
첫 빌드는 시간이 오래 걸리며, Rust 크레이트와 웹 SDK를 내려받기 위해 네트워크 연결이 필요하다.

## 네이티브 빠른 시작

rustup이 설치되어 있으면 저장소에서 Rust 명령을 실행할 때 고정 툴체인과 필요한
target을 자동으로 준비한다. 먼저 버전을 확인하고 앱을 실행한다.

```sh
rustc --version
cargo --version
cargo run
```

기본 빌드는 검은 배경만 지우는 빈 창을 띄우며, 창을 닫으면 종료한다.
저장소에 포함된 자체 제작 스프라이트 예제는 다음 명령으로 재생한다.

```sh
cargo run -- --play tests/fixtures/example.sprites.json
```
개발 도구를 쓰려면 `dev` feature를 켜서 다시 실행한다.

```sh
cargo run --features dev
```

dev 빌드도 검은 창으로 시작한다. 백틱 키(`Grave`, 보통 Esc 아래)를 누르면 상단 메뉴바와
`Frame stats`가 함께 나타나고, 다시 누르면 모두 숨는다. `View` 메뉴에서는
`Dear ImGui Demo`와 `Dear ImGui Metrics`를 여닫을 수 있으며, 처음에는 `Frame stats`만 켜져
있다. 창을 닫거나 숨겨도 그 상태는 프로세스가 살아 있는 동안만 유지하고 디스크에는 저장하지 않는다.
키 반복 입력, Ctrl·Alt·GUI 조합 키, 텍스트 입력 중 누른 백틱에는 도구가 반응하지 않는다.
라벨은 ASCII 영어이며 한글 폰트가 없어 한글은 `?`로 표시된다.

네이티브 dev의 `Assets > Asset Library`에서는 PNG 가져오기, 격자·타일·sprite·animation 편집,
미리보기, 영역 후보 찾기, 타일셋 추출을 할 수 있다. 저장 형식과 사용 순서는
[스프라이트 안내](sprites.md)를 참고한다. `Guess`는 수동으로 적용하는 격자 제안 기능이며, 큰 맵에서는 약 55초 동안 멈춘 사례가 있다.

feature는 빌드 프로필과 독립적이므로 `cargo run --release --features dev`로도 실행할 수 있다.
일반 실행은 자동으로 종료되지 않으므로, 화면이 없는 환경에서는 다음 자동 검사를 사용한다.

```sh
cargo test --locked
cargo test --locked --features dev
scripts/smoke-native.sh
scripts/smoke-native.sh --dev
scripts/check-no-imgui.sh
```

일반 테스트만으로는 디스플레이 검사를 대신할 수 없다.
스모크 판정 기준과 코드 구조는 [네이티브 개발과 검사](development.md)에서 설명한다.

## 웹 빠른 시작

```sh
scripts/setup-emsdk.sh
scripts/with-emsdk.sh emcc --version
scripts/build-web.sh
scripts/serve-web.sh
```

서버를 띄운 터미널을 그대로 두고 브라우저에서 `http://127.0.0.1:8080/`을 연다.
기본 빌드는 검은 canvas만 표시한다. 확인이 끝나면 터미널에서 Ctrl-C로 서버를 종료한다.
개발 도구가 필요하면 `scripts/build-web.sh --dev`로 다시 빌드한 뒤 서버를 띄운다.
네이티브와 마찬가지로 백틱 키와 `View` 메뉴를 사용한다. `target/web/`의 산출물은 항상 마지막 빌드가 덮어쓴다.
다른 터미널에서 다음 명령으로 빌드한 페이지를 자동 검사할 수 있다.

```sh
scripts/smoke-web.sh
```

dev 빌드 뒤에는 `scripts/smoke-web.sh --dev`를 실행한다. 기대 모드와 빌드 모드가
다르면 검사가 실패하므로, 스모크 실행 직전에 반드시 같은 모드로 빌드해야 한다.

같은 서버에서 `http://127.0.0.1:8080/?play=example`을 열면 코드로 생성한 예제를 재생하며,
`&animation=cycle`로 재생할 animation을 지정할 수 있다. 웹에서는 편집·추출·저장을 지원하지 않는다.
비dev 빌드 뒤 `scripts/smoke-web.sh --play`를 실행하면 재생까지 검사한다.

SDK 공유 위치, 포트 변경, release 빌드, 오류 진단은
[웹 실행과 문제 해결](web.md)을 참고한다. 웹 빌드는 upstream이 공식 지원하지 않는 로컬 패치
경로를 사용하며, 자세한 이유는 [의존성과 패치 관리](dependencies.md)에서 설명한다.
현재 단계에서는 로컬 HTTP 서버만 다루고 웹 배포는 하지 않는다.

## 더 읽기와 CI 상태

- [스프라이트 시트 안내](sprites.md): JSON 사이드카, 편집·추출, 네이티브·웹 재생.
- [네이티브 개발과 검사](development.md): `App`, 드라이버, 종료, 테스트와 스모크 출력.
- [웹 실행과 문제 해결](web.md): SDK 설치, 빌드 산출물, 서버와 브라우저 증거.
- [의존성과 패치 관리](dependencies.md): 버전 정본, 승인된 패치, 판 올림 절차.

CI 설정 파일은 `.github/workflows/ci.yml`이며, 연동된 origin 저장소에 push하거나 pull request를 열 때마다
실행된다. 2026-11-08 develop 브랜치 CI는 성공했다. v0.3.0의 스프라이트 형식·편집·추출,
dev·비dev 검사, 재생 스모크는 통합 브랜치에서 로컬로 확인했다.
이번 에픽의 원격 CI는 아직 실행하지 않았으며 origin push도 하지 않았다.
CI 역시 로컬과 같은 스모크 스크립트를 사용하며, 웹 비dev 빌드에 `--play` 검사를 추가로 실행한다.

Decided in: isk-srq5, isk-35g9, isk-7w7b, isk-fcuz
