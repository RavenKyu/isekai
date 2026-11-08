# 네이티브 개발과 검사

[개발 시작](README.md)에서 도구를 준비한 뒤 사용하는 실행·검사 안내다.
웹 드라이버는 [웹 실행](web.md), 라이브러리와 버전 정책은
[의존성과 패치](dependencies.md)를 참고한다. 명령은 checkout 최상단 기준이다.

## 창 실행과 종료

```sh
cargo run
```

기본 빌드는 SDL3 창과 OpenGL 3.3 core 컨텍스트로 검은 배경만 지운다.
imgui 의존성과 UI 코드는 포함하지 않는다. 개발 도구가 필요하면 앞의 창을 닫고
feature를 켜서 실행한다.

```sh
cargo run --features dev
```

dev도 처음에는 검은 창이다. 물리 백틱 키(scancode `Grave`, 보통 Esc 아래)를 누르면
메뉴바와 `Frame stats`가 나타난다. 다시 누르면 메뉴바와 개발 창이 모두 숨는다.
`Frame stats`에는 FPS와 프레임 시간(ms)이 나오며 값이 유효하지 않으면 `--`로 표시한다.
`View` 메뉴의 `Frame stats`, `Dear ImGui Demo`, `Dear ImGui Metrics`는 창 상태와
연결된 체크 항목이다. 처음에는 stats만 열려 있고 Demo와 Metrics는 닫혀 있다.
닫기 버튼이나 메뉴로 각 창을 여닫을 수 있고 전체를 숨겼다 열어도 상태가 유지된다.

키 반복은 토글하지 않는다. Ctrl·Alt·GUI 조합도 무시하며 Shift 조합은 허용한다.
ImGui가 텍스트 입력을 원할 때는 백틱을 문자로 입력하고 도구를 숨기지 않는다.
라벨은 ASCII 영어다. 한글 폰트가 없어 한글 글리프는 `?`로 보인다.
창 배치나 설정을 `imgui.ini`에 저장하지 않으며 Info/Settings 창은 아직 없다.

`dev`는 프로필과 별개다. 최적화한 개발 도구도 다음처럼 실행할 수 있다.

```sh
cargo run --release --features dev
```

네이티브 앱은 `cargo run`으로 실행한다. `target/debug/rusty-nails`를 직접 실행하면
시스템 `libSDL3`에 연결될 수 있다. 창 닫기나 SDL 종료 이벤트로 루프를 끝낸다.
두 모드 모두 종료 전에 GL 컨텍스트를 current로 만들며, dev는 renderer, platform
순서로 종료를 시도한다. 무인자 실행에는 프레임 수 제한이 없다.

화면이 있는 환경에서 자동 검사를 하고 결과를 보려면 다음을 실행한다.

```sh
cargo run --locked -- --smoke-frames 3
cargo run --locked --features dev -- --smoke-frames 3
```

스모크는 첫 프레임 뒤 키를 자동으로 보내므로 dev에서 사람이 키를 누를 필요가 없다.
디스플레이가 없는 서버에서는 아래 스모크 스크립트를 사용한다. 유효한 창이나
GL 컨텍스트가 없으면 실패하며 검사 생략이나 성공으로 취급하지 않는다.

## 코드가 나뉘는 곳

| 파일 | 역할 |
| --- | --- |
| `src/main.rs` | 네이티브 반복 루프와 Emscripten RAF callback 드라이버. feature에 따라 스모크 기대 모드를 정하고 `App::frame`을 부른다. |
| `src/app.rs` | SDL·GL 소유, 이벤트 처리, 화면 지우기, 스모크 키 주입/도착 관측과 framebuffer 측정, 종료. |
| `src/dev.rs` | dev 전용 renderer·platform·ImGui 소유, 모든 이벤트 전달, 백틱 토글 판정과 프레임 렌더링. |
| `src/ui.rs` | dev 전용 메뉴바와 창 상태, FPS·프레임 시간 표시, 안전한 Demo/Metrics 창. |
| `src/smoke.rs` | 플랫폼·feature와 독립적인 인자 해석, 숨김/열림과 시간 제한 판정, 결과 한 줄 생성. |
| `tests/smoke_native.rs` | 실제 앱 subprocess로 두 모드 렌더링, 인자 오류, 디스플레이 실패, 일반 실행 지속을 검사. |
| `scripts/smoke-native.sh` | 독립 Xvfb와 software GL, 최대 60초 제한으로 통합 검사와 종료 이벤트 테스트를 실행. |
| `scripts/check-no-imgui.sh` | 두 target 의존성 그래프와 새 비dev 실행 파일 기호를 검사. |

`App::new`는 `Box<App>`을 돌려준다. dev 계층 필드를 GL 컨텍스트와 창보다 먼저 선언하고,
계층 안에서는 renderer·platform·ImGui 순으로 해제한다. SDL은 그 뒤까지 살아 있다.
초기화 실패 때도 renderer를 먼저 종료한다. 네이티브는 `App::shutdown`을 호출한다.
웹은 힙의 `App`을 페이지 수명 동안 보유하고 스모크 완료 시 루프만 취소해 마지막
canvas를 남긴다. 해제 순서는 코드로 확인했으며 계측 테스트는 rn-rj8j의 범위다.

## 평소 검사 순서

```sh
cargo check --locked
cargo check --locked --features dev
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --all-targets --features dev -- -D warnings
cargo test --locked
cargo test --locked --features dev
scripts/check-no-imgui.sh
scripts/smoke-native.sh
scripts/smoke-native.sh --dev
shellcheck scripts/*.sh tests/shell/*.sh
tests/shell/emsdk_test.sh
```

일반 테스트는 두 모드 모두 스모크 인자와 상태 기계를 검사한다. dev에서는 추가로
메뉴와 각 창의 정점, 숨김 시작과 상태 보존, 키 충돌, 실제 ImGui 텍스트 입력 요청,
FPS 문자열을 CPU에서 검사한다. 화면이 필요한 종료 이벤트와 통합 스모크는 기본적으로
ignore되어 있으므로 두 스모크 명령을 이어서 실행한다.

스모크 스크립트는 SDL 종료와 창 닫기를 별도 프로세스에서 각각 검사한다.
X11·software GL과 새 Xvfb를 강제해 기존 데스크톱 세션에 붙지 않는다.
58초 TERM 제한과 2초 강제 정리 여유를 두어 전체 상한은 60초다.
각 앱 subprocess 감시는 10초다. 첫 컴파일이 제한에 들어가지 않게 두 feature의
일반 테스트를 먼저 빌드한다. `--dev`는 세 Cargo test 호출 모두에 같은 feature를 넘긴다.
알 수 없는 인자나 중복 인자는 사용법과 종료 코드 2를 반환한다.

`check-no-imgui.sh`는 host와 Emscripten의 normal/build 의존성 그래프에서 비dev에
imgui가 없고 dev에는 있는지 양성 대조까지 확인한다. 이어 비dev 바이너리를 새로 빌드하고
`nm -C`로 imgui 기호가 없음을 확인한다. 기호가 지워졌거나 앱 기호가 없으면 실패한다.
release 기호도 확인하려면 다음을 사용한다.

```sh
scripts/check-no-imgui.sh --release
```

셸 테스트는 가짜 emsdk Git 저장소로 동시 설치, 중단 복구, worktree와 공백 경로,
버전 교체 및 인자 보존을 검사한다. 실제 SDK·웹 검사를 대신하지 않는다.
[웹 스모크](web.md#웹-스모크)로 이어 간다. 회귀 검사를 바꿀 때는
[실패하는 변이로 약속을 확인하는 교훈](lessons/verify-promises-with-mutations.md)을 따른다.

## 스모크 출력 계약

성공 시 앱이 내는 한 줄은 다음 형식이다.

```text
rn-smoke: ok frames=<N> vtx=<M> lit=<K>
```

`N`은 요청한 양의 프레임 수, `M`은 마지막으로 센 프레임의 ImGui 정점 수,
`K`는 그 프레임에서 검은 clear 색과 다른 RGB 픽셀 수다.
픽셀은 렌더 후 swap 전에 읽으며 alpha만으로 그렸다고 판단하지 않는다.

| 모드 | 시작·키 입력과 프레임 수 | 성공 판정 |
| --- | --- | --- |
| dev | 먼저 최소 한 숨김 프레임을 확인하고 실제 백틱으로 연다. 열리기 전 모든 프레임은 `vtx=0`, 열림 대기는 5초 이내다. `frames=N`은 열린 뒤 센 수이며 도중에 닫히면 실패한다. | 마지막 프레임 `M>0`, `K>0`. |
| 비dev | 첫 빈 프레임 뒤 백틱이 도착해도 UI가 생기지 않아야 한다. 키 도착을 관측한 뒤 N 프레임을 센다. 키가 5초 안에 안 오면 실패한다. | `vtx=0 lit=0`. |

네이티브는 첫 프레임 뒤 SDL 큐에 비반복 KeyDown, 반복 KeyDown, KeyUp을 한 번 넣는다.
웹은 [브라우저의 실제 키 경로](web.md#웹-스모크)를 쓴다. 비dev의 검은 픽셀 판정은
그리기를 발견하지만 clear 호출 자체가 빠진 경우까지 보장하지 않는다.

실패 형식은 `rn-smoke: fail <이유>`다. 결과 줄 끝에는 개행 하나만 붙인다.
네이티브 앱과 검사 스크립트는 실패 시 0이 아닌 종료 코드를 준다.
웹 앱은 페이지에 결과를 쓰고 브라우저 스크립트가 결과와 콘솔·네트워크 오류를 읽어
종료 코드로 바꾼다. 전체 로그에는 Cargo나 브라우저 진단도 섞일 수 있다.
한 줄 계약은 앱 결과에 적용하며 네이티브 테스트는 subprocess stdout 전체를 맞춘다.

## 실행이 안 될 때

- SDL3 빌드 오류: C/C++ 도구와 창 시스템 개발 패키지를 확인한다. 첫 소스 빌드가
  오래 걸리는 것과 실패를 구분한다.
- 창·GL 초기화 오류: 그래픽 세션과 OpenGL 3.3 드라이버를 확인한다.
  headless 환경은 `scripts/smoke-native.sh`와 `--dev`로 검사한다.
- 개발 도구가 안 보임: `--features dev`로 빌드했는지 확인한 뒤 물리 백틱을 누른다.
  텍스트 입력 포커스를 놓고 Ctrl·Alt·GUI를 떼어 다시 시도한다.
- `dev tools did not open within 5s`: 실제 키 도착, scancode와 반복 판정을 확인한다.
  `smoke key probe did not arrive within 5s`는 비dev에서도 키가 큐에 도착하지 않은 경우다.
- 라벨이 `?`: 한글 폰트는 포함하지 않으므로 ASCII 라벨을 사용한다.
- dev의 `vtx=0`·`lit=0`: UI나 렌더 호출이 빠졌는지 확인한다. 비dev의 `lit>0`은
  검은 배경 외 그리기가 있다는 뜻이다. 모드를 확인하고 임계값을 낮추지 않는다.
- `missing required tool`: 오류에 나온 Xvfb, Python, timeout, nm 등을 준비한다.

Decided in: rn-35g9, rn-7w7b
