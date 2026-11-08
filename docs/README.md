# Rusty Nails 개발 시작

Rust와 SDL3로 네이티브 창과 WebGL 2 앱을 만든다. Dear ImGui 개발 도구는
Cargo의 `dev` feature를 켰을 때만 포함된다. 기본 개발 창은 두 플랫폼에서 공유하고,
스프라이트 편집·추출은 네이티브 dev에서 제공한다.
이 문서는 Linux에서 개발 환경을 준비하고 두 실행 경로를 확인하는 안내다.
모든 명령은 전달받은 저장소 checkout의 최상단에서 실행한다.

## 필요한 것

검증 환경은 Ubuntu 26.04 계열이다. Windows와 macOS의 설치 절차는 다루지 않는다.

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
`libxi-dev`, `libxfixes-dev`, `libxkbcommon-dev`이며 GL 헤더는 `libegl-dev`,
`libgl-dev`로 준비할 수 있다. Wayland를 쓰려면 해당 개발 패키지도 필요하다.
SDL3의 다른 선택 기능에서 요구하는 라이브러리는 CMake 로그를 확인한다.
시스템 SDL3 패키지를 미리 설치할 필요는 없다. Cargo가 SDL3를 소스로 빌드하므로
첫 빌드는 오래 걸리며, Rust 크레이트와 웹 SDK 다운로드를 위한 네트워크도 필요하다.

## 네이티브 빠른 시작

rustup이 설치되어 있으면 저장소에서 Rust 명령을 실행할 때 고정 툴체인과 필요한
target을 준비한다. 먼저 버전을 확인하고 앱을 실행한다.

```sh
rustc --version
cargo --version
cargo run
```

기본 빌드는 배경만 검게 지우는 빈 창이다. 창을 닫으면 종료한다.
저장소의 자체 제작 스프라이트 예제는 다음으로 재생한다.

```sh
cargo run -- --play tests/fixtures/example.sprites.json
```
개발 도구를 쓰려면 다음처럼 feature를 켜서 다시 실행한다.

```sh
cargo run --features dev
```

dev도 검은 창으로 시작한다. 백틱 키(`Grave`, 보통 Esc 아래)를 누르면 상단 메뉴바와
`Frame stats`가 함께 나타나고 다시 누르면 모두 숨는다. `View` 메뉴에서
`Dear ImGui Demo`와 `Dear ImGui Metrics`를 여닫는다. 처음에는 `Frame stats`만 켜져
있다. 창을 닫거나 숨겨도 그 상태는 프로세스 안에서 유지하며 디스크에는 저장하지 않는다.
키 반복 입력, Ctrl·Alt·GUI 조합 키, 텍스트 입력 중 누른 백틱은 도구를 토글하지 않는다.
라벨은 ASCII 영어이며 한글 폰트가 없어 한글은 `?`로 보인다.

네이티브 dev의 `Assets > Sprite Library`에서 PNG 가져오기, 격자·타일·sprite·clip 편집,
미리보기, 영역 후보와 타일셋 추출을 한다. [스프라이트 안내](sprites.md)에 저장 형식과
사용 순서가 있다. `Guess`는 수동 적용하는 제안이며 큰 맵에서는 약 55초 멈춘 사례가 있다.

feature는 빌드 프로필과 별개라 `cargo run --release --features dev`도 가능하다.
일반 실행은 저절로 종료되지 않는다. 화면 없는 환경에서는 다음 자동 검사를 사용한다.

```sh
cargo test --locked
cargo test --locked --features dev
scripts/smoke-native.sh
scripts/smoke-native.sh --dev
scripts/check-no-imgui.sh
```

일반 테스트만으로 디스플레이 검사를 통과한 것은 아니다.
스모크의 판정과 코드 구조는 [네이티브 개발과 검사](development.md)에 있다.

## 웹 빠른 시작

```sh
scripts/setup-emsdk.sh
scripts/with-emsdk.sh emcc --version
scripts/build-web.sh
scripts/serve-web.sh
```

마지막 명령을 실행한 터미널을 둔 채 `http://127.0.0.1:8080/`을 브라우저로 연다.
기본 빌드는 검은 canvas다. 확인한 뒤 터미널에서 Ctrl-C로 서버를 종료한다.
개발 도구가 필요하면 `scripts/build-web.sh --dev`로 다시 빌드하고 서버를 띄운다.
네이티브와 같은 백틱 키와 `View` 메뉴를 쓴다. `target/web/`의 내용은 마지막 빌드가 덮어쓴다.
다른 터미널에서는 빌드한 페이지를 자동으로 검사할 수 있다.

```sh
scripts/smoke-web.sh
```

dev 빌드 뒤에는 `scripts/smoke-web.sh --dev`를 사용한다. 기대 모드와 빌드 모드가
다르면 검사에 실패하므로 각 스모크 실행 직전에 같은 모드로 빌드한다.

같은 서버에서 `http://127.0.0.1:8080/?play=example`을 열면 코드 생성 예제를 재생한다.
`&clip=cycle`로 clip을 지정할 수 있다. 웹에는 편집·추출·저장이 없다.
비dev 빌드 뒤 `scripts/smoke-web.sh --play`로 재생까지 검사한다.

SDK 공유 위치, 다른 포트, release 빌드와 오류 진단은
[웹 실행과 문제 해결](web.md)에 있다. 웹은 upstream이 지원하지 않는 로컬 패치
경로를 사용하며, [의존성과 패치 관리](dependencies.md)에서 그 이유를 설명한다.
이 단계에서는 로컬 HTTP 서버만 다루고 웹 배포는 하지 않는다.

## 더 읽기와 CI 상태

- [스프라이트 라이브러리](sprites.md): JSON 사이드카, 편집·추출, 네이티브·웹 재생.
- [네이티브 개발과 검사](development.md): `App`, 드라이버, 종료, 테스트와 스모크 출력.
- [웹 실행과 문제 해결](web.md): SDK 설치, 빌드 산출물, 서버와 브라우저 증거.
- [의존성과 패치 관리](dependencies.md): 버전 정본, 승인된 패치, 판 올림 절차.

CI 파일은 `.github/workflows/ci.yml`이다. origin 저장소가 연동되어 있어 push와 pull request마다
실행한다. 2026-11-08 develop CI는 성공했다. v0.3.0의 스프라이트 형식·편집·추출,
dev·비dev 검사와 재생 스모크는 통합 브랜치에서 로컬로 확인했다.
이 에픽의 원격 CI는 미실행이며 origin push도 하지 않았다.
CI도 로컬과 같은 스모크 스크립트를 쓰고 웹 비dev 빌드에 `--play` 검사를 추가한다.

Decided in: rn-35g9, rn-7w7b, rn-fcuz
