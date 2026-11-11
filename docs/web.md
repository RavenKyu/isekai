# 웹 실행과 문제 해결

같은 `App`을 `wasm32-unknown-emscripten`으로 빌드해 SDL3·glow로 WebGL 2에 그린다.
기본 빌드는 검은 canvas만 표시하며, Dear ImGui 개발 도구는 `dev` feature를 켰을 때만 포함된다. 먼저 [개발 시작](README.md)의 도구를 준비한다.
이 경로는 upstream 공식 지원 경로가 아니므로 [의존성과 패치](dependencies.md)에
설명한 로컬 crate 패치 두 개가 필요하다. 명령은 checkout 최상단 기준이다.


## isekai의 wasm-bindgen 웹 경로

새 앱은 `wasm32-unknown-unknown` target을 사용한다. 아래 명령은 기존 Emscripten 경로와 나란히
사용할 수 있으며, 기존 `build-web.sh` 산출물을 덮어쓰지 않는다.

```sh
scripts/setup-wasm-bindgen.sh
scripts/build-web-isekai.sh --dev
scripts/smoke-web-isekai.sh --dev
scripts/build-web-isekai.sh
scripts/smoke-web-isekai.sh
scripts/smoke-web-isekai.sh --play
scripts/smoke-web-isekai.sh --webgl2
scripts/smoke-web-isekai.sh --no-webgpu
scripts/serve-web-isekai.sh
```

빌드 산출물은 `target/web-isekai/{index.html,isekai.js,isekai_bg.wasm}`에 모인다.
서버는 loopback 주소와 `PORT`(기본값 8080, 0이면 빈 포트 자동 선택)를 사용한다.
CLI 버전의 정본은 `Cargo.lock`에 잠긴 `wasm-bindgen` 버전이다. 주 checkout의
`.tools/wasm-bindgen/<version>/` 아래에 `flock` 잠금으로 한 번만 설치하며, 실행 파일의 실제 버전과 완료 표시 파일을
검사한다. 완료 표시가 없으면 중단된 설치로 보고 다시 설치한다. 외부 `PATH`의 CLI는 쓰지 않으며,
완료된 설치의 버전이 정본과 다르면 실패한다.

dev 빌드는 egui 기본 폰트와 OFL 나눔고딕 Regular를 wasm에 내장한다. OS 폰트나 별도 폰트
다운로드 없이 한글 음절 전체를 표시하며, 비dev wasm에는 폰트가 들어가지 않는다.
브라우저 클립보드는 연결하지 않았으므로 페이지 밖에서 복사한 글은 붙여 넣을 수 없다.
`--dev` 스모크는 실제 canvas readback에서 두 family의 Latin·한글을 대체 글리프와 비교한다.
검사 띠와 실패 판정은 [egui 셸 폰트](development.md#egui-셸-폰트)에서 설명한다.

지원하는 URL 쿼리는 `play=example`, `animation=cycle`(`clip` 별칭 허용), `smoke=3`,
`backend=auto|webgpu|webgl2`다. `auto`는 WebGPU 어댑터·장치 요청이 실패하면
새 GL 인스턴스를 만들어 WebGL2로 재시도한다. Rust 진입점이 URL 쿼리를 CLI와 동일한 clap 인자로 변환한다.
코드로 생성한 `example` 시트만 내장하며, 웹에서는 파일 편집·저장을 지원하지 않는다.

`isekai-smoke:` 결과는 `#isekai-log` 요소와 브라우저 콘솔에 함께 기록한다. 비동기 초기화와 첫 프레임
렌더링을 마친 뒤 `data-ready=true` 속성을 붙이고, CDP를 통해 실제 `Backquote` 키 이벤트를 받는다.
픽셀 읽기는 다음 이벤트 루프 틱에서 완료 콜백을 받으며, 측정이 끝난 뒤에도 마지막 canvas 화면을 유지한다.
웹에서는 블로킹 GPU 대기나 `std::time::Instant`를 쓰지 않는다.

자동 검사는 `auto` 모드에서 `backend=webgpu` 또는 `gl`을 허용하고, WebGL2 강제(`--webgl2`) 및 WebGPU 비활성화(`--no-webgpu`)
모드에서는 `gl`만 허용한다. 준비 신호가 나오기 전에 실패하더라도 `dom.html`, `screenshot.png`, `console.json`, `chrome.log`,
`server.log`를 `target/smoke-web-isekai.<임의값>/`에 보존한다. 잘못된 모드 조합이나 중복 인자는 종료 코드 2를 반환한다.


## SDK 설치와 worktree 공유

```sh
scripts/setup-emsdk.sh
scripts/with-emsdk.sh emcc --version
```

emsdk 버전 정본은 `.emsdk-version` 파일 한 줄이다. 설치 위치는 **주 checkout의** `.tools/emsdk`
한 곳이며, 연결된 모든 linked worktree가 이를 공유한다. 별도 clone 저장소는 각자의 `.tools/emsdk`를
쓴다. `scripts/emsdk-common.sh`가 `git-common-dir`로 주 checkout 경로를 찾으므로
worktree마다 SDK를 중복 설치하거나 경로를 하드코딩하지 않는다.

설치는 `.tools/emsdk.lock`의 배타적 `flock` 잠금 안에서 진행하며, 설치·활성화·실제 compiler
버전 검증까지 모두 통과해야 `.rn-installed`에 버전을 기록한다. 완료 표시 파일이 없거나 기록된 버전이 pin과 다르면
같은 잠금 안에서 설치 디렉터리만 지우고 새로 설치한다. 설치가 중간에 끊겼을 때도 `setup-emsdk.sh`를 다시
실행하면 복구된다. 이미 같은 pin으로 설치가 끝난 상태에서 다시 실행해도 안전하다.

`with-emsdk.sh`는 명령이 끝날 때까지 공유 잠금을 유지해 실행 도중 SDK가 교체되는 일을 막는다.
SDK가 설치되어 있지 않으면 자동으로 내려받지 않고 setup 스크립트 실행을 안내한다. 외부 `PATH`에 있는 다른
`emcc`를 임의로 쓰지 않으며, 실행 파일 경로와 실제 버전을 검증한 뒤 인자를 원래대로 분리해
전달한다. SDK 환경 변수는 해당 명령에만 적용하고 사용자의 셸 설정 파일은 건드리지 않는다.

## 빌드와 산출물

```sh
scripts/build-web.sh
```

웹은 스프라이트 재생만 지원한다. `src/sprite/embedded.rs`의 예제 표가 코드로 생성한 RGBA 이미지와
메타데이터를 제공하며, 외부 파일 preload나 아티스트 이미지 임베드는 하지 않는다. 서버를 띄운 뒤
`http://127.0.0.1:8080/?play=example` 또는 `?play=example&animation=cycle`을 열면 재생된다.
`animation`을 생략하면 이름순 첫 animation을 고른다. 구버전 링크 호환을 위해 `clip=`도 `animation=`의 별칭으로 받는다. URL의 `play` 값은 파일 경로가 아니라 내장 예제 이름이다.
자세한 형식은 [스프라이트 안내](sprites.md#웹-재생)를 참고한다.

기본 빌드는 debug·비dev 모드이며 `target/web/`에 `index.html`, `rusty-nails.js`, `rusty-nails.wasm`을
모은다. 복사하기 전 원본 산출물은 `target/wasm32-unknown-emscripten/debug/`에 생성된다.
Cargo가 만드는 WASM 파일 이름은 `rusty_nails.wasm`이고 JS는 `rusty-nails.js`인데,
빌드 스크립트가 WASM을 `rusty-nails.wasm`으로 이름을 맞춰 복사하며 HTML의 `Module.locateFile`도
이 이름을 가리킨다. 따라서 `web/index.html`만 따로 복사해서는 앱이 실행되지 않는다.

개발 도구나 최적화 빌드가 필요할 때는 다음 옵션을 조합해 빌드한다.

```sh
scripts/build-web.sh --dev
scripts/build-web.sh --release
scripts/build-web.sh --release --dev
```

`--dev`는 Cargo에 `--features dev`를 넘긴다. `--release`와 `--dev`는 서로 독립적이므로
`--dev --release` 순서로 주어도 된다. 각 인자는 한 번씩만 쓸 수 있으며, 중복 인자나
알 수 없는 인자는 사용법 안내와 함께 종료 코드 2를 반환한다.

dev 모드 역시 검은 canvas로 시작한다. 물리 백틱 키(`Grave`)로 상단 메뉴바와 `Frame stats`를
함께 여닫고, `View` 메뉴의 체크 항목 세 개로 `Frame stats`·Demo·Metrics를 각각 여닫는다.
처음에는 `Frame stats`만 열려 있다. 키 반복 입력, Ctrl·Alt·GUI 조합 키, 텍스트 입력 중 누른 백틱에는
도구가 토글되지 않는다. 도구 전체를 숨겨도 개별 창의 열림 상태는 유지된다. 라벨은 ASCII 영어이며 한글 폰트는 제공하지 않는다.

`--release`를 주면 원본 경로는 `target/wasm32-unknown-emscripten/release/`로 바뀌지만, 최종 staging 디렉터리는 여전히
`target/web/`이다. 즉, 마지막에 실행한 빌드가 `target/web/` 안의 debug/release 및 dev/비dev 산출물을 덮어쓰며,
서로 다른 모드의 산출물을 동시에 보관하지 않는다.
빌드 스크립트는 외부 `CARGO_TARGET_DIR` 환경 변수와 무관하게 항상 현재 checkout의 `target/`을
사용한다. target별 링크 플래그는 `.cargo/config.toml`에서 관리하며 LTO는 켜지 않는다.

## 로컬 서버

```sh
scripts/serve-web.sh
```

브라우저에서 `http://127.0.0.1:8080/`을 연다. `file://` 경로로는 열지 않는다.
서버는 `127.0.0.1`에만 바인딩하며 **현재 checkout의 `target/web/` 디렉터리만** 서빙한다.
저장소 루트 전체를 노출하거나 `0.0.0.0`에 바인딩하지 않는다. 서버를 끄려면 터미널에서 Ctrl-C를 누른다.
빌드 산출물이 없으면 먼저 빌드하라는 오류를 낸다.

다른 포트를 지정하거나 OS가 빈 포트를 자동으로 고르게 할 수도 있다.

```sh
PORT=8081 scripts/serve-web.sh
```

```sh
PORT=0 scripts/serve-web.sh
```

각 예시는 기존 서버를 종료한 뒤 실행한다. `PORT=0`을 주었을 때는 Python이 출력하는
`Serving HTTP on 127.0.0.1 port ...` 줄에서 실제 포트 번호를 확인해 접속한다.
이 문서는 로컬 실행만 다루며 GitHub Pages 등 외부 웹 배포는 범위 밖이다.

## 웹 스모크

스모크 검사는 직전 빌드와 같은 모드로 실행해야 한다. 별도로 띄워 둔 로컬 서버는 필요 없다.

```sh
scripts/build-web.sh --dev
scripts/smoke-web.sh --dev
scripts/build-web.sh
scripts/smoke-web.sh
scripts/smoke-web.sh --play
```

`smoke-web.sh [--dev|--play]`는 직접 빌드를 수행하지 않고 현재 `target/web/`에 있는 산출물을 검사한다.
빌드 모드와 다른 모드로 검사하면 실패한다. 중복 인자나 알 수 없는 인자는
종료 코드 2를 반환한다. 인자 없음(빈 화면), `--dev`(개발 UI), `--play`(비dev 재생) 셋 중
하나만 선택할 수 있으며 `--dev --play`나 중복 인자는 허용하지 않는다. 내부 재실행 시에도 선택한 인자를
그대로 유지하며, 요청 모드가 유실되면 즉시 실패한다. `--play`는 같은 비dev 빌드에 대해
`?smoke=3&play=example`을 열며 별도의 재생 전용 빌드 명령은 없다.

`CHROME` 환경 변수를 지정하지 않으면 `google-chrome`, `chromium` 순서로 브라우저를 찾는다.
실행 파일 경로를 직접 지정할 때는 다음과 같이 실행한다.

```sh
CHROME=/usr/bin/google-chrome scripts/smoke-web.sh
```

위 경로는 Chrome이 그곳에 설치된 환경의 예시이므로 실제 환경에 설치된 브라우저 경로로 바꿔 쓴다.
스모크 스크립트는 실행할 때마다 독립된 브라우저 프로필과 `PORT=0` 임시 서버를 띄우고,
headless Chrome의 SwiftShader 백엔드로 `?smoke=3` 페이지를 연다. Python 표준 라이브러리만으로
DevTools pipe를 제어하므로 npm이나 외부 Python 패키지는 필요 없다.

`Module.postRun`은 `App::new` 초기화와 메인 루프 등록을 마친 뒤 `rn-log` 요소에
`data-ready="true"` 속성을 붙인다. 이 준비 신호는 결과 텍스트를 건드리지 않는다.
드라이버는 `data-ready="true"`를 확인한 뒤 `requestAnimationFrame`을 두 번 기다려 최소 한 프레임이
지나간 것을 보장한다. 그다음 canvas에 포커스를 주고 CDP `Input.dispatchKeyEvent`로
`Backquote`의 keyDown, autoRepeat keyDown, keyUp을 세 검사 모드 모두 동일하게 한 번 보낸다.
앱 코드 안에서 웹 개발 도구를 강제로 열지는 않는다.

dev 모드는 도구가 열리기 전 모든 프레임에서 `vtx=0`인지 확인하고, 실제 키 입력으로 도구가 열린 뒤 3프레임을 센다.
5초 안에 열리지 않거나 프레임을 세는 도중 다시 닫히면 실패한다.
정상적인 dev 결과는 `rn-smoke: ok frames=3 vtx=<양수> lit=<양수>`다.
비dev 모드는 이벤트 큐에 키가 도착한 것을 확인한 뒤 3프레임을 세며, 결과는
`rn-smoke: ok frames=3 vtx=0 lit=0`이어야 한다. 비dev 재생(`--play`) 역시 같은 키 프로브 규칙을 따르며
결과는 `rn-smoke: ok frames=3 vtx=0 lit=<양수>`여야 한다. 키 도착 대기 제한도 5초다.
공통 앱의 마지막 프레임 측정값뿐 아니라 JS/WASM의 HTTP 200 응답 여부, 페이지 오류,
콘솔 오류, 네트워크 요청 실패도 함께 검사한다. [출력 계약](development.md#rn-smoke-출력-계약)은
네이티브와 동일하며, 1회 실행의 전체 시간 상한은 정리 여유를 포함해 60초다.

stderr에 출력되는 `evidence:` 경로는 `target/smoke-web.<임의값>/`이다.
그 안의 `dom.html`, `screenshot.png`, `chrome.log`, `console.json`, `server.log`에서
실제 페이지 상태와 실패 원인을 확인할 수 있다. 스크립트가 임시 서버와 브라우저를 종료한 뒤에도
이 증거 파일들은 삭제하지 않고 남긴다.

## 문제 해결

| 증상 | 확인할 것 |
| --- | --- |
| SDK 미설치·완료 표시 불일치 | `scripts/setup-emsdk.sh`를 다시 실행한다. 공유 SDK를 다른 명령이 사용 중이면 잠금 해제를 기다린다. |
| emcc 위치나 버전 불일치 | pin 파일과 `scripts/with-emsdk.sh emcc --version` 결과를 확인한다. 외부 SDK를 직접 source하는 대신 wrapper를 사용한다. |
| C++ 또는 WASM 링크 실패 | `.cargo/config.toml`과 toolchain pin을 확인한다. `DEFAULT_TO_CXX`와 예외 방식·LLVM 조합을 임의로 바꾸지 않는다. |
| WASM 404·빈 canvas | build-web을 다시 실행하고 `target/web`의 세 파일, HTTP 요청, `Module.locateFile` 이름을 확인한다. `file://`는 사용하지 않는다. |
| `no embedded sprite sheet named ...`·`no animation named ...` | `play=example`, `animation=cycle`을 확인한다. 네이티브 파일 경로는 웹 임베드 이름이 아니다. |
| `lit=0: sprite playback drew nothing` | `--play` URL 전달, 재생 draw와 코드 예제 색을 확인한다. Blank 기대값으로 바꾸지 않는다. |
| 주소 사용 중 | 앞의 서버를 종료하거나 `PORT=0`으로 새 빈 포트를 배정한다. |
| `missing browser`·`CHROME not executable` | headless를 지원하는 Chrome/Chromium을 준비하고 실제 경로를 지정한다. |
| dev 모드의 `lit=0`, 페이지·콘솔 오류 | evidence의 DOM·스크린샷·console.json·server.log를 확인한다. 렌더링 오류나 404를 무시하고 성공 처리하지 않는다. |
| `missing or malformed smoke result`·모드 불일치 | 마지막 빌드가 dev인지 확인하고 build와 smoke 양쪽에 같은 `--dev`를 지정한다. rn-log에는 결과 한 줄만 있어야 한다. |
| `dev tools did not open within 5s`·`smoke key probe did not arrive within 5s` | evidence와 준비 속성, canvas 포커스, CDP 키가 SDL에 도착하는지 확인한다. 고정 sleep이나 앱 내부의 강제 표시로 우회하지 않는다. |
| `missing smoke result`·DevTools timeout | 결과 줄이 없는지, 앱이 초기화·RAF·렌더 중 멈췄는지 증거 로그를 본다. 페이지가 응답하지 않아 DevTools도 멈추면 DOM과 스크린샷은 남지 않고 `chrome.log`와 `server.log`만 남는다. CLI `--dump-dom`만으로 완료 시점을 추정하지 않는다. |

Decided in: isk-kznl, isk-srq5, isk-35g9, isk-7w7b, isk-fcuz
