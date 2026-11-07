# 네이티브 개발과 검사

[개발 시작](README.md)에서 도구를 준비한 뒤 사용하는 실행·검사 안내다.
웹 드라이버는 [웹 실행](web.md), 라이브러리와 버전 정책은
[의존성과 패치](dependencies.md)를 참고한다. 명령은 checkout 최상단 기준이다.

## 창 실행과 종료

```sh
cargo run
```

SDL3 창과 OpenGL 3.3 core 컨텍스트에서 `Hello World`를 그린다. 창 닫기나 SDL
종료 이벤트를 받으면 루프를 끝내고 GL 컨텍스트가 current인 상태에서 renderer,
platform 순서로 종료를 시도한다. 무인자 실행에는 프레임 수 제한이 없다.

화면이 있는 환경에서 세 프레임을 그린 뒤 종료 결과를 보려면 다음을 실행한다.

```sh
cargo run --locked -- --smoke-frames 3
```

디스플레이가 없는 서버에서는 아래 스모크 스크립트를 사용한다. 유효한 창이나
GL 컨텍스트가 없으면 실패하며, 그 상태를 검사 생략이나 성공으로 취급하지 않는다.

## 코드가 나뉘는 곳

| 파일 | 역할 |
| --- | --- |
| `src/main.rs` | 네이티브 반복 루프와 Emscripten RAF callback 드라이버. 둘 다 `App::frame`을 부른다. |
| `src/app.rs` | SDL·GL·ImGui와 backend 소유, 이벤트 처리, 프레임 렌더링, framebuffer 측정, 종료. |
| `src/ui.rs` | 두 플랫폼이 함께 쓰는 `Hello World` UI. |
| `src/smoke.rs` | `--smoke-frames` 인자 해석, 마지막 프레임 판정, 성공·실패 한 줄 생성. |
| `tests/smoke_native.rs` | 실제 앱 subprocess를 실행해 렌더링, 인자 오류, 디스플레이 실패, 일반 실행 지속을 검사. |
| `scripts/smoke-native.sh` | 독립 Xvfb와 software GL, 최대 60초 제한으로 위 통합 검사와 종료 이벤트 테스트를 실행. |

`App::new`는 `Box<App>`을 돌려준다. 필드 선언 순서로 renderer·platform·ImGui가
GL 컨텍스트와 창보다 먼저 해제되며 SDL은 그 뒤까지 살아 있다. 네이티브 드라이버는
`App::shutdown`을 호출한다. 웹은 callback이 스택을 빌리지 않도록 힙의 `App`을
페이지 수명 동안 보유하고, 스모크 완료 시 루프만 취소해 마지막 canvas를 남긴다.

## 평소 검사 순서

```sh
cargo check --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
scripts/smoke-native.sh
shellcheck scripts/*.sh tests/shell/*.sh
tests/shell/emsdk_test.sh
```

`cargo test --locked`는 CPU에서 UI 텍스트의 정점 증가와 스모크 인자·판정을 검사한다.
화면이 필요한 종료 이벤트 테스트와 네이티브 통합 스모크는 기본적으로 ignore되어
있다. `scripts/smoke-native.sh`가 둘 다 실행하므로 반드시 이어서 돌린다. 종료 이벤트
테스트는 SDL 종료 이벤트와 창 닫기를 프로세스를 나눠 각각 검사한다.
이 스크립트는 X11·software GL을 강제하고 새 Xvfb를 사용해 기존 데스크톱 세션에
붙지 않는다. 58초 TERM 제한과 2초 강제 정리 여유를 두어 전체 상한은 60초다.
첫 빌드가 이 제한에 걸리지 않게 먼저 일반 빌드·테스트를 끝낸다.

셸 테스트는 네트워크 없는 가짜 emsdk Git 저장소로 동시 설치, 중단 복구, worktree와
공백 경로, 버전 교체 및 명령 인자 보존을 검사한다. 실제 SDK 설치나 웹 렌더링
검사를 대신하지는 않는다. 웹 검사는 [웹 스모크](web.md#웹-스모크)에서 이어 간다.

## 스모크 출력 계약

성공 시 앱이 내는 한 줄은 다음 형식이다.

```text
rn-smoke: ok frames=<N> vtx=<M> lit=<K>
```

`N`은 요청한 양의 프레임 수, `M`은 마지막 프레임의 ImGui 정점 수,
`K`는 그 프레임에서 검은 clear 색과 다른 RGB 픽셀 수다. `M`과 `K`가 모두 양수여야
성공이다. 픽셀은 렌더링 후 swap 전에 읽으며 alpha만으로 그렸다고 판단하지 않는다.
네이티브와 웹은 같은 줄을 사용하고 끝에 개행 하나를 붙인다.

실패 형식은 `rn-smoke: fail <이유>`다. 네이티브 앱과 검사 스크립트는 실패 시
0이 아닌 종료 코드를 준다. 웹 앱은 페이지에 결과를 쓰므로 브라우저 스크립트가
그 결과와 콘솔·네트워크 오류를 읽어 성공/실패 종료 코드로 바꾼다.

스모크 스크립트의 전체 로그에는 Cargo나 브라우저 진단도 섞일 수 있다.
한 줄 계약은 앱의 결과 줄에 적용된다. 네이티브 통합 테스트는 성공한 subprocess의
stdout 전체가 정확한 한 줄인지 검사하고, 실패 상황이 실제로 실패하는지도 확인한다.

## 실행이 안 될 때

- SDL3 빌드 중 오류: C/C++ 도구와 X11 또는 Wayland 개발 패키지를 확인한다.
  첫 소스 빌드 시간이 길어지는 것과 컴파일 실패를 구분한다.
- 창·GL 초기화 오류: 로그인한 그래픽 세션과 OpenGL 3.3 드라이버를 확인한다.
  headless 환경은 `scripts/smoke-native.sh`로 검사한다.
- `missing required tool`: 오류에 나온 Xvfb, Python, timeout 등을 설치한 뒤 다시 검사한다.
- `vtx=0` 또는 `lit=0`: 각각 UI geometry가 없거나 framebuffer가 clear 색뿐이다.
  `src/ui.rs`와 실제 렌더 호출을 확인하며, 검사를 통과시키려고 임계값을 낮추지 않는다.

Decided in: rn-35g9
