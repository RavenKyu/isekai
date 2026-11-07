# Rusty Nails 개발 시작

Rust, SDL3, Dear ImGui로 같은 `Hello World`를 네이티브 창과 브라우저에 그린다.
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

`rusty-nails` 창 안의 `Hello` 창에 `Hello World`가 보이면 창을 닫는다.
일반 실행은 계속 그리므로 저절로 종료되지 않는다. 화면 없는 환경에서는
일반 창 대신 다음 자동 검사를 사용한다.

```sh
cargo test --locked
scripts/smoke-native.sh
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
같은 `Hello World`가 보이는지 확인한 뒤 터미널에서 Ctrl-C로 서버를 종료한다.
다른 터미널에서는 빌드한 페이지를 자동으로 검사할 수 있다.

```sh
scripts/smoke-web.sh
```

SDK 공유 위치, 다른 포트, release 빌드와 오류 진단은
[웹 실행과 문제 해결](web.md)에 있다. 웹은 upstream이 지원하지 않는 로컬 패치
경로를 사용하며, [의존성과 패치 관리](dependencies.md)에서 그 이유를 설명한다.
이 단계에서는 로컬 HTTP 서버만 다루고 웹 배포는 하지 않는다.

## 더 읽기와 CI 상태

- [네이티브 개발과 검사](development.md): `App`, 드라이버, 종료, 테스트와 스모크 출력.
- [웹 실행과 문제 해결](web.md): SDK 설치, 빌드 산출물, 서버와 브라우저 증거.
- [의존성과 패치 관리](dependencies.md): 버전 정본, 승인된 패치, 판 올림 절차.

CI 파일의 위치는 `.github/workflows/ci.yml`이다. 이 에픽에서는 git remote가 없어
GitHub Actions의 실제 실행을 확인하지 못했다. 로컬 검사 성공과 원격 CI 성공을
구분해야 한다. CI가 사용하는 검사도 로컬의 같은 스크립트가 입구다.

Decided in: rn-35g9
