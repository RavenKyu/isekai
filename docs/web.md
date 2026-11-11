# 웹 실행과 문제 해결

같은 `App`을 `wasm32-unknown-emscripten`으로 빌드해 SDL3·glow로 WebGL 2에 그린다.
기본 빌드는 검은 canvas만 표시하며, Dear ImGui 개발 도구는 `dev` feature를 켰을 때만 포함된다. 먼저 [개발 시작](README.md)의 도구를 준비한다.
이 경로는 upstream 공식 지원 경로가 아니며 [의존성과 패치](dependencies.md)에
설명한 두 로컬 crate 패치가 필요하다. 명령은 checkout 최상단 기준이다.

## SDK 설치와 worktree 공유

```sh
scripts/setup-emsdk.sh
scripts/with-emsdk.sh emcc --version
```

버전 정본은 `.emsdk-version` 한 줄이다. 설치 위치는 **주 checkout의** `.tools/emsdk`
하나이며 모든 linked worktree가 이를 공유한다. 별도 clone은 자기 `.tools/emsdk`를
쓴다. `scripts/emsdk-common.sh`가 `git-common-dir`로 주 checkout을 찾으므로
worktree마다 SDK를 복제하거나 경로를 하드코딩하지 않는다.

설치는 `.tools/emsdk.lock`의 배타 `flock` 안에서 진행하고, 설치·활성화·실제 compiler
버전 검증이 끝나야 `.rn-installed`에 버전을 쓴다. 완료 표시가 없거나 pin과 다르면
같은 잠금 안에서 설치 디렉터리만 지우고 다시 설치한다. 중단 뒤에도 setup을 다시
실행하면 된다. 같은 pin의 완료된 설치에는 재실행이 안전하다.

`with-emsdk.sh`는 공유 잠금을 명령 실행 끝까지 유지해 사용 중 SDK 교체를 막는다.
설치가 없으면 자동 다운로드하지 않고 setup 실행을 요청한다. 외부 PATH의 다른
`emcc`를 조용히 쓰지 않으며, 실행 파일 위치와 실제 버전을 확인한다. 인자는 분리된
상태로 전달한다. SDK 환경은 그 명령에만 적용하며 셸 시작 파일을 수정하지 않는다.

## 빌드와 산출물

```sh
scripts/build-web.sh
```

웹은 재생만 지원한다. `src/sprite/embedded.rs`의 예제 표가 코드로 생성한 RGBA 이미지와
메타데이터를 제공하며 파일 preload나 사람 이미지 임베드는 없다. 서버 실행 후
`http://127.0.0.1:8080/?play=example` 또는 `?play=example&animation=cycle`로 재생한다.
animation을 생략하면 이름 순 첫 animation을 쓴다. 옛 링크의 `clip=`은 `animation=`의 별칭이다. URL의 `play`는 파일 경로가 아니라 임베드 이름이다.
자세한 형식은 [스프라이트 안내](sprites.md#웹-재생)를 따른다.

기본은 debug·비dev이며 `target/web/`에 `index.html`, `rusty-nails.js`, `rusty-nails.wasm`을
모은다. 복사 전 원본은 `target/wasm32-unknown-emscripten/debug/`에 있다.
Rust가 내는 WASM 파일은 `rusty_nails.wasm`이고 JS는 `rusty-nails.js`다.
빌드 스크립트가 WASM을 `rusty-nails.wasm`으로 복사하고 HTML의 `Module.locateFile`도
이 이름을 가리킨다. `web/index.html`만 따로 복사하면 앱이 실행되지 않는다.

개발 도구나 최적화 빌드는 다음 중 필요한 조합으로 빌드한다.

```sh
scripts/build-web.sh --dev
scripts/build-web.sh --release
scripts/build-web.sh --release --dev
```

`--dev`는 Cargo의 `--features dev`로 전달한다. release와 dev는 서로 독립이며
`--dev --release` 순서로 지정해도 된다. 각 인자는 한 번만 지정할 수 있으며, 중복이나
알 수 없는 인자는 사용법 안내와 함께 종료 코드 2를 반환한다.

dev 모드도 검은 canvas로 시작한다. 물리 백틱 키(`Grave`)로 메뉴바와 `Frame stats`를
함께 열고 닫는다. `View` 메뉴의 세 체크 항목으로 stats·Demo·Metrics를 각각 여닫는다.
처음에는 stats만 열려 있다. 키 반복 입력, Ctrl·Alt·GUI 조합 키, 텍스트 입력 중 누른 백틱은
도구를 토글하지 않는다. 창을 숨겨도 열림 상태는 유지된다. 라벨은 ASCII 영어이며 한글 폰트는 제공하지 않는다.

원본은 `target/wasm32-unknown-emscripten/release/`로 바뀌지만 staging 위치는 여전히
`target/web/`이다. 마지막 빌드가 그 폴더의 debug/release 및 dev/비dev 내용을 덮어쓴다.
서로 다른 모드의 산출물을 동시에 보관하지 않는다.
빌드 스크립트는 외부 `CARGO_TARGET_DIR` 값과 관계없이 해당 checkout의 `target/`을
사용한다. target 링크 플래그는 `.cargo/config.toml`에 모으며 LTO는 켜지 않는다.

## 로컬 서버

```sh
scripts/serve-web.sh
```

브라우저에서 `http://127.0.0.1:8080/`을 연다. `file://`로 열지 않는다.
서버는 `127.0.0.1`에만 bind하고 **해당 checkout의 `target/web/`만** 제공한다.
저장소 전체나 `0.0.0.0`으로 범위를 넓히지 않는다. 종료는 서버 터미널의 Ctrl-C다.
빌드가 없는 경우 서버는 먼저 빌드하라는 오류를 낸다.

다른 포트를 지정하거나 OS가 빈 포트를 정하게 할 수도 있다.

```sh
PORT=8081 scripts/serve-web.sh
```

```sh
PORT=0 scripts/serve-web.sh
```

위 예시는 각각 앞의 서버를 종료한 뒤 실행한다. `PORT=0`일 때는 Python의
`Serving HTTP on 127.0.0.1 port ...` 출력에서 실제 포트를 읽어 접속한다.
이 안내는 로컬 실행만 다루며 GitHub Pages 등의 웹 배포는 범위 밖이다.

## 웹 스모크

빌드와 스모크 검사의 모드를 일치시킨다. 이미 실행 중인 수동 서버는 필요하지 않다.

```sh
scripts/build-web.sh --dev
scripts/smoke-web.sh --dev
scripts/build-web.sh
scripts/smoke-web.sh
scripts/smoke-web.sh --play
```

`smoke-web.sh [--dev|--play]`는 빌드를 직접 수행하지 않고 현재 `target/web/`의 산출물을 검사한다.
모드가 일치하지 않는 빌드를 검사하면 실패한다. 중복되거나 알 수 없는 인자는
종료 코드 2를 반환한다. 무인자(빈 화면), `--dev`(개발 UI), `--play`(비dev 재생) 중
하나를 고른다. `--dev --play`나 중복 인자는 허용하지 않는다. 재실행도 선택한 인자를
보존하고 요청 모드가 유실되면 명시적으로 실패한다. `--play`는 같은 비dev 빌드의
`?smoke=3&play=example`을 검사하며 별도 재생 빌드 명령은 없다.

`CHROME`을 주지 않으면 `google-chrome`, `chromium` 순서로 찾는다.
실행 파일을 명시할 때는 다음과 같이 쓴다.

```sh
CHROME=/usr/bin/google-chrome scripts/smoke-web.sh
```

이 경로는 Chrome이 그곳에 설치된 환경의 예시다. 다른 환경에서는 설치된 실행
파일의 경로로 바꾼다. 스모크는 실행마다 독립 프로필과 `PORT=0` 서버를 만들고,
headless Chrome의 SwiftShader로 `?smoke=3` 페이지를 연다. Python 표준 라이브러리로
DevTools pipe를 사용하므로 npm이나 별도 Python 모듈은 필요하지 않다.

`Module.postRun`은 `App::new` 생성과 루프 등록이 끝난 뒤 `rn-log` 요소에
`data-ready="true"`를 붙인다. 준비 신호는 결과 텍스트를 바꾸지 않는다.
드라이버는 이 속성을 기다린 뒤 `requestAnimationFrame`을 두 번 기다려 최소 한 번의 앱
프레임이 지난 것을 보장한다. canvas에 포커스를 맞추고 CDP `Input.dispatchKeyEvent`로
`Backquote` keyDown, autoRepeat keyDown, keyUp을 세 검사 모드 모두 한 번 보낸다.
앱 코드가 웹 도구를 강제로 열지 않는다.

dev 모드는 열리기 전 모든 프레임의 `vtx=0`을 확인하고 실제 키로 열린 뒤 3프레임을 센다.
5초 안에 열리지 않거나 센 프레임 도중 다시 닫히면 실패한다.
정상 dev 결과는 `rn-smoke: ok frames=3 vtx=<양수> lit=<양수>`다.
비dev 모드는 큐의 키 도착을 확인한 뒤 3프레임을 세며
`rn-smoke: ok frames=3 vtx=0 lit=0`이어야 한다. 비dev 재생(`--play`)은 같은 프로브 규칙에
`rn-smoke: ok frames=3 vtx=0 lit=<양수>`여야 한다. 키 도착 대기도 5초 이내다.
공통 앱의 마지막 프레임 측정뿐 아니라 JS/WASM HTTP 200, 페이지 오류,
콘솔 오류, 네트워크 실패도 검사한다. [출력 계약](development.md#스모크-출력-계약)은
네이티브와 같다. 한 번의 실행 상한은 정리 여유를 포함해 60초다.

stderr에 출력되는 `evidence:` 경로는 `target/smoke-web.<임의값>/`다.
`dom.html`, `screenshot.png`, `chrome.log`, `console.json`, `server.log`에서
실제 페이지와 실패 이유를 확인한다. 스크립트가 자신의 서버·브라우저를 종료해도
이 증거 파일은 남는다.

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

Decided in: isk-35g9, isk-7w7b, isk-fcuz
