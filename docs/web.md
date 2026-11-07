# 웹 실행과 문제 해결

같은 `App`을 `wasm32-unknown-emscripten`으로 빌드해 SDL3·Dear ImGui·glow로
WebGL 2에 그린다. 먼저 [개발 시작](README.md)의 도구를 준비한다.
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

기본은 debug이며 `target/web/`에 `index.html`, `rusty-nails.js`, `rusty-nails.wasm`을
모은다. 복사 전 원본은 `target/wasm32-unknown-emscripten/debug/`에 있다.
Rust가 내는 WASM 파일은 `rusty_nails.wasm`이고 JS는 `rusty-nails.js`다.
빌드 스크립트가 WASM을 `rusty-nails.wasm`으로 복사하고 HTML의 `Module.locateFile`도
이 이름을 가리킨다. `web/index.html`만 따로 복사하면 앱이 실행되지 않는다.

최적화 빌드가 필요하면 다음을 사용한다.

```sh
scripts/build-web.sh --release
```

원본은 `target/wasm32-unknown-emscripten/release/`로 바뀌지만 staging 위치는 여전히
`target/web/`다. 마지막 빌드가 그 폴더의 debug/release 내용을 바꾼다.
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

빌드를 마친 뒤 실행한다. 이미 실행 중인 수동 서버는 필요하지 않다.

```sh
scripts/smoke-web.sh
```

`CHROME`을 주지 않으면 `google-chrome`, `chromium` 순서로 찾는다.
실행 파일을 명시할 때는 다음과 같이 쓴다.

```sh
CHROME=/usr/bin/google-chrome scripts/smoke-web.sh
```

이 경로는 Chrome이 그곳에 설치된 환경의 예시다. 다른 환경에서는 설치된 실행
파일의 경로로 바꾼다. 스모크는 실행마다 독립 프로필과 `PORT=0` 서버를 만들고,
headless Chrome의 SwiftShader로 `?smoke=3` 페이지를 연다. Python 표준 라이브러리로
DevTools pipe를 사용하므로 npm이나 별도 Python 모듈은 필요하지 않다.

정상 결과는 `rn-smoke: ok frames=3 vtx=<양수> lit=<양수>`다.
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
| 주소 사용 중 | 앞의 서버를 종료하거나 `PORT=0`으로 새 빈 포트를 배정한다. |
| `missing browser`·`CHROME not executable` | headless를 지원하는 Chrome/Chromium을 준비하고 실제 경로를 지정한다. |
| `lit=0`, 페이지·콘솔 오류 | evidence의 DOM·스크린샷·console.json·server.log를 확인한다. 렌더링 오류나 404를 무시하고 성공 처리하지 않는다. |
| `missing smoke result`·DevTools timeout | 결과 줄이 없는지, 앱이 초기화·RAF·렌더 중 멈췄는지 증거 로그를 본다. 페이지가 응답하지 않아 DevTools도 멈추면 DOM과 스크린샷은 남지 않고 `chrome.log`와 `server.log`만 남는다. CLI `--dump-dom`만으로 완료 시점을 추정하지 않는다. |

Decided in: rn-35g9
