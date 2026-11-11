# 네이티브 개발과 검사

[개발 시작](README.md)에서 도구를 준비한 뒤 사용하는 실행·검사 안내다.
웹 드라이버는 [웹 실행](web.md), 라이브러리와 버전 정책은
[의존성과 패치](dependencies.md)를 참고한다. 명령은 checkout 최상단 기준이다.


## isekai 워크스페이스와 검사

`crates/isekai-core`는 CPU 이미지·시트 형식·저장·시간표와 순수 편집 연산을 공유한다.
`isekai_core::sprite::edit`의 `SpriteSheet` 메서드는 ID·modified·검증 순서를 유지하며,
선택과 기하 UI helper는 기존 `src/ui/asset_library/edit.rs`에 남는다.
`crates/isekai-render`는 wgpu 배치·프레임 타깃·픽셀 읽기·표면 전달을 맡고,
`crates/isekai-editor`는 egui 셸, `crates/isekai`는 CLI·winit 루프·웹 진입점을 맡는다.

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo clippy --locked -p rusty-nails --all-targets --features dev -- -D warnings
cargo clippy --locked -p isekai --all-targets --features dev -- -D warnings
cargo test --locked --workspace
cargo test --locked -p rusty-nails --features dev
cargo test --locked -p isekai --features dev
scripts/check-no-egui.sh
scripts/smoke-native-isekai.sh
scripts/smoke-native-isekai.sh --dev
```

루트에서 `cargo test`만 실행하면 옮긴 코어 테스트가 돌지 않는다. dev 검사 역시 각 패키지를
명시한다. 새 앱의 해제 순서는 egui → 렌더 자원 → Surface → Arc 창이다.
모든 장면은 `Rgba8Unorm` 오프스크린 타깃에 그린 뒤 표면 전달 패스 한 번을 거친다.
픽셀 읽기는 256바이트 행 정렬 패딩을 제거하고 RGB가 검정이 아닌 픽셀만 센다.

비dev `isekai`의 의존성 그래프와 기호에는 egui가 없어야 하며, dev 그래프에는 있어야 한다.
모든 `isekai` 그래프에서 SDL3와 dear-imgui는 금지하되, wgpu GL 백엔드의 glow는 허용한다.
egui `Context`는 인스턴스별 상태이므로 기존 ImGui처럼 전역 컨텍스트 검사를 자식 프로세스로 격리할 필요가 없다.
진단 창은 파일 저장 경로를 연결하지 않으며 persistence·clipboard feature도 켜지 않는다.
기존 `scripts/check-no-imgui.sh`와 `rusty-nails` 네이티브 스모크도 계속 실행한다.

### egui 셸 폰트

egui 0.37은 기본 feature에 폰트를 포함하지 않는다. workspace 의존성에서 `default_fonts`를
명시해 Latin·기호를 복구하고, `isekai-editor::fonts::context()`가 내장 나눔고딕 Regular를
Proportional·Monospace 양쪽의 마지막 fallback으로 붙인다. 한글 음절 11,172자와 붙여 넣은
한글 문자열을 표시할 수 있으며, IME 조합은 지원 범위에 포함하지 않는다.
이 fallback은 한글을 고정폭으로 바꾸지는 않는다. OS 폰트 설치나 실행 중 다운로드는 필요 없다.
자산의 OFL 전문·출처·버전·SHA256은 [폰트 기록](../assets/fonts/nanum-gothic/README.md)에 있다.

```sh
cargo test --locked -p isekai-editor
scripts/smoke-native-isekai.sh --dev
scripts/build-web-isekai.sh --dev
scripts/smoke-web-isekai.sh --dev
```

단위 검사는 설정된 두 family에서 `has_glyph`로 Latin·기호와 한글 음절 전체를 조회하며,
대체 글리프를 성공으로 세지 않는다. dev 스모크는 실제 셸을 연 뒤 별도 라벨 띠에
`A`, `é`, `가`, `힣`과 없는 문자 U+10FFFF를 그린다. 동일한 GPU framebuffer에서
각 문자의 흰색 픽셀 mask를 그 family의 대체 글리프 mask와 비교하므로 빈 네모도 실패한다.
이 검사는 네이티브·웹의 공통 readback 경로에서 수행한다. 일반 실행에는 검사 띠가 나오지 않는다.
`é`는 기본 폰트, `가`·`힣`은 한글 fallback의 누락을 잡는다.
실패는 `isekai-smoke: fail glyph probe: ...`로 보고한다.

폰트는 optional editor 안에서만 `include_bytes!`로 넣는다. `scripts/check-no-egui.sh`는
두 target의 그래프와 네이티브 기호 검사에 더해, 새 비dev native/wasm 산출물에 저장소의
폰트 원본 bytes가 포함되지 않았는지 검사한다.

### isekai-smoke 출력 계약

성공 시 stdout에 `isekai-smoke: ok frames=N vtx=M lit=K backend=B` 한 줄을 출력하고,
실패 시 `isekai-smoke: fail reason`을 출력한 뒤 0이 아닌 종료 코드로 끝난다. `backend`는
`vulkan`, `metal`, `dx12`, `gl`, `webgpu`, `unknown` 중 하나다. 진단 로그는 stderr로 출력한다.

| 모드 | vtx | lit |
| --- | --- | --- |
| 비dev 빈 화면 | 0 | 0 |
| 비dev 재생 | 0 | 양수 |
| dev | 양수 | 양수 |

하네스는 `window-mapped` 신호가 나온 뒤 창 안으로 포인터를 옮기고 포커스를 준다.
winit이 `Focused(true)`를 처리하고 첫 프레임을 표시하면 stderr에 `isekai-smoke: ready`가 출력된다.
이 신호를 확인한 뒤 `xdotool` XTEST로 실제 백틱 키 눌림·뗌을 한 번 보낸다. 별도의 장기 입력 검사는
자동 반복을 켠 Xvfb에서 약 800ms 동안 키를 누른 채 유지하며 `repeat=true` 도착, 토글 1회, 키 해제를
확인한다. `isekai-smoke-input:`은 stderr에 `presses/repeats/releases/toggles`를 기록한다.
포커스를 얻을 때 winit이 만드는 합성 키 스냅샷은 새 눌림으로 세지 않으며 egui에는 전달한다.
창 생성·포커스·첫 프레임·키·토글 단계 진단은 스모크 모드에서만 stderr에 남긴다. 비dev 모드도 키 프로브가 도착해야 프레임을 센다.
키 미도착이나 dev 미열림 제한 시간은 5초이며, 디스플레이나 어댑터가 없으면 실패한다.
네이티브 스모크는 독립된 Xvfb와 `WGPU_BACKEND=vulkan`,
`VK_LOADER_DRIVERS_SELECT='*lvp*'`로 lavapipe를 고정하며 전체 제한 시간 60초를 둔다.
일반 `--smoke-frames` 실행에서도 외부의 실제 키 입력이 필요하다.


## 창 실행과 종료

```sh
cargo run
```

기본 빌드는 SDL3 창과 OpenGL 3.3 core 컨텍스트로 검은 배경만 지우며,
imgui 의존성과 UI 코드는 포함하지 않는다. 개발 도구가 필요하면 창을 닫고
`dev` feature를 켜서 다시 실행한다.

```sh
cargo run --features dev
```

dev 빌드도 처음에는 검은 창으로 시작한다. 물리 백틱 키(scancode `Grave`, 보통 Esc 아래)를 누르면
메뉴바와 `Frame stats`가 나타나고, 다시 누르면 메뉴바와 개발 창이 모두 숨는다.
`Frame stats`에는 FPS와 프레임 시간(ms)을 표시하며 값이 유효하지 않으면 `--`로 나타낸다.
`View` 메뉴의 `Frame stats`, `Dear ImGui Demo`, `Dear ImGui Metrics`는 각 창의 열림 상태와
연동된 체크 항목이다. 처음에는 `Frame stats`만 열려 있고 Demo와 Metrics는 닫혀 있다.
네이티브 dev의 `Assets > Asset Library`는 PNG 시트 편집·추출 창이다.
가져오기·저장 순서와 `Guess`의 진행률 표시·취소 방법은 [스프라이트 사용법](sprites.md#편집기-사용)에서 설명한다.
닫기 버튼이나 메뉴로 각 창을 개별적으로 여닫을 수 있으며, 도구 전체를 숨겼다 다시 열어도 각 창의 열림 상태는 유지된다.

키를 길게 누를 때 발생하는 반복 입력에는 토글하지 않는다. Ctrl·Alt·GUI 조합 키도 무시하며 Shift 조합은 허용한다.
ImGui가 텍스트 입력을 요청할 때는 백틱을 일반 문자로 입력하고 도구를 숨기지 않는다.
라벨은 ASCII 영어이며 한글 폰트가 없어 한글 글리프는 `?`로 보인다.
창 배치나 설정을 `imgui.ini`에 저장하지 않는다. Demo 창의 `Log To File`을 눌러도 `imgui_log.txt`를
만들지 않으며 Info/Settings 창은 아직 없다.

`dev` feature는 빌드 프로필과 별개이므로 최적화된 개발 도구도 다음처럼 실행할 수 있다.

```sh
cargo run --release --features dev
```

네이티브 앱은 `cargo run`으로 실행한다. `target/debug/rusty-nails`를 직접 실행하면
시스템의 `libSDL3`에 연결될 수 있다. 창 닫기나 SDL 종료 이벤트가 들어오면 메인 루프를 끝낸다.
두 모드 모두 종료 전에 GL 컨텍스트를 current로 설정하며, dev 모드는 라이브러리 텍스처의
외부 ID 해제·삭제, renderer, platform 순서로 종료를 시도한다. 인자 없이 실행하면 프레임 수 제한 없이 계속 돈다.

화면이 있는 환경에서 자동 검사를 실행하고 결과를 직접 확인하려면 다음 명령을 쓴다.

```sh
cargo run --locked -- --smoke-frames 3
cargo run --locked --features dev -- --smoke-frames 3
cargo run --locked -- --play tests/fixtures/example.sprites.json cycle --smoke-frames 3
```

스모크 검사는 첫 프레임 이후 키 이벤트를 자동으로 넣으므로 dev 모드에서 사람이 직접 키를 누를 필요가 없다.
디스플레이가 없는 서버에서는 아래 스모크 스크립트를 사용한다. 유효한 창이나
GL 컨텍스트를 만들지 못하면 실패로 처리하며, 검사를 건너뛰거나 성공으로 취급하지 않는다.

## 코드가 나뉘는 곳

| 파일 | 역할 |
| --- | --- |
| `src/test_support.rs` | `cfg(test)` 전용 공유 도우미: 임시 디렉터리, 숨은 SDL·GL 창 픽스처, 정확한 테스트 경로로 자식 프로세스를 다시 실행하고 종료 성공과 `1 passed`를 확인하는 래퍼. |
| `src/main.rs` | 네이티브 반복 루프와 Emscripten RAF callback 드라이버. feature에 따라 스모크 기대 모드를 정하고 `App::frame`을 호출한다. |
| `src/app.rs` | SDL·GL 소유, 이벤트 처리, 화면 지우기, 스모크 키 주입/도착 관측과 framebuffer 측정, 종료. |
| `src/dev.rs` | dev 전용 renderer·platform·ImGui 소유, 모든 이벤트 전달, 백틱 토글 판정과 프레임 렌더링. |
| `src/ui.rs` | dev 전용 메뉴바와 창 상태, FPS·프레임 시간 표시, 안전한 Demo/Metrics 창. |
| `crates/isekai-core/src/sprite/{grid,format,store,timeline,example}.rs` | 격자·v3 형식(v1·v2 읽기)·경로/원자적 저장·재생 시간표·코드 생성 예제와 fixture. |
| `src/sprite/{texture,draw,playback,embedded}.rs` | GL 텍스처·셀 단위 쿼드·네이티브/웹 재생·코드 예제 표. |
| `crates/isekai-core/src/raster.rs`, `crates/isekai-core/src/raster/analysis.rs` | CPU PNG·RGBA·crop과 격자 추정·정규화·dedupe·rebuild·8방향 영역 분석. |
| `src/ui/asset_library/` | 네이티브 dev의 가져오기·작업 큐·격자·타일·sprite·animation·미리보기·Regions·Extract. |
| `src/smoke.rs` | 플랫폼·feature와 무관한 인자 해석, 숨김/열림과 제한 시간 판정, 결과 한 줄 생성. |
| `tests/smoke_native.rs` | 실제 앱 subprocess로 두 모드의 렌더링, 인자 오류, 디스플레이 실패, 일반 실행 지속을 검사. |
| `scripts/smoke-native.sh` | 독립 Xvfb와 software GL, 최대 60초 제한으로 통합 검사와 종료 이벤트 테스트를 실행. |
| `scripts/check-no-imgui.sh` | 두 target의 의존성 그래프와 새로 빌드한 비dev 실행 파일 기호를 검사. |

`App::new`는 `Box<App>`을 반환한다. 구조체 필드 선언 순서에 따라 dev 계층과 `Playback`의 텍스처·shader가
GL 컨텍스트와 창보다 먼저 해제된다. dev 계층 안에서는 편집기 텍스처, renderer, platform, ImGui
순으로 해제되며 SDL은 그 뒤까지 유지된다. Load/Reload/Close/Replace 큐는 매 프레임
GL 컨텍스트를 current로 만든 뒤 `platform.new_frame`을 호출하기 전에 처리한다.
초기화 도중 실패하더라도 renderer를 먼저 종료한다. 네이티브 드라이버는 종료 시 `App::shutdown`을 호출한다.
웹은 힙에 할당한 `App`을 페이지 수명 동안 유지하며, 스모크가 끝나면 루프만 취소해 마지막
canvas 화면을 남긴다. 텍스처 업로드·Drop·외부 ID 해제는 GL 테스트로 검증한다.
네이티브 GL readback 검사는 `SpriteQuad`의 회전·반전 방향도 함께 확인한다. 측정하지 않는 범위는 [스프라이트 검사](sprites.md#검사)에 정리되어 있다.
`App` 필드 순서나 전체 shutdown 순서가 바뀌는 변이를 자동으로 잡아내는 검사는 없으므로,
이 순서는 코드 리뷰에서 직접 확인해야 한다. 전체 수명 계측은 isk-rj8j의 작업 범위다.

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

일반 테스트는 두 모드 모두 스모크 인자 해석과 상태 기계를 검사한다. dev 모드에서는 추가로
메뉴와 각 창의 정점 수, 숨김 상태 시작과 열림 상태 보존, 키 충돌, 실제 ImGui 텍스트 입력 요청,
FPS 문자열 포맷을 CPU에서 검사한다. 디스플레이가 필요한 종료 이벤트 테스트와 통합 스모크는 기본적으로
`#[ignore]` 처리되어 있으므로 두 스모크 명령을 이어서 실행한다.

스모크 스크립트는 fixture 기반 재생의 성공·실패도 함께 검사한다. 예제 PNG/JSON을 `target/` 아래에
복사한 뒤, 없는 사이드카·없는 animation·`../` 이미지 경로를 주었을 때 `rn-smoke: fail`과 0이 아닌 종료 코드로 끝나는지 확인한다.
SDL 종료와 창 닫기, GL 큐 검사는 SDL 초기화 스레드가 충돌하지 않도록 각각 별도 프로세스에서 실행한다.
X11·software GL과 새 Xvfb를 강제해 기존 데스크톱 세션에 붙지 않도록 격리한다.
58초 TERM 제한과 2초 강제 정리 여유를 두어 전체 실행 상한은 60초다.
개별 앱 subprocess의 감시 제한 시간은 10초다. 첫 컴파일 시간이 실행 제한에 걸리지 않도록 두 feature의
일반 테스트 바이너리를 먼저 빌드한다. `--dev`를 주면 세 번의 `cargo test` 호출 모두에 같은 feature를 넘긴다.
알 수 없는 인자나 중복 인자는 사용법 안내와 함께 종료 코드 2를 반환한다.

`check-no-imgui.sh`는 host와 Emscripten의 normal/build 의존성 그래프에서 비dev에는
imgui·rfd가 없고 host dev에는 둘 다 있는지 대조 확인한다.
웹 dev 그래프에는 rfd가 없어야 한다. 이어서 비dev 바이너리를 새로 빌드하고
`nm -C`로 imgui·rfd 기호가 없음을 확인한다. 기호가 strip되었거나 앱 기호 자체가 없으면 실패로 처리한다.
release 바이너리의 기호까지 확인하려면 다음 명령을 사용한다.

```sh
scripts/check-no-imgui.sh --release
```

셸 테스트는 가짜 emsdk Git 저장소를 만들어 동시 설치, 중단 후 복구, worktree와 공백 포함 경로,
버전 교체, 인자 보존을 검사한다. 이 테스트가 실제 SDK·웹 검사를 대신하지 않으므로
[웹 스모크](web.md#웹-스모크)까지 이어서 실행한다. 회귀 검사를 수정할 때는
[실패하는 변이로 약속을 확인하는 교훈](lessons/verify-promises-with-mutations.md)을 따른다.
파일 열기·저장 경로를 수정할 때는 [검증한 경로 해석 결과를 그대로 사용하는 규칙](lessons/use-the-path-that-was-validated.md)을 확인한다.
ImGui 테스트에는 [기본 실행과 ignored 실행을 모두 자식으로 격리하는 규칙](lessons/isolate-imgui-tests-at-every-entry.md)을 적용한다.
성공 경로 테스트에서는 [fixture의 전제조건](lessons/prepare-valid-fixtures-for-success-paths.md)을 먼저 맞춘다.
파일 재열기 방식을 바꿀 때는 [쓰기 경로와 파일 식별 기준](lessons/inspect-write-paths-when-deduplicating-open-files.md)도 함께 점검한다.

## rn-smoke 출력 계약

기존 `rusty-nails` 스모크가 성공하면 앱은 다음 형식으로 한 줄을 출력한다.

```text
rn-smoke: ok frames=<N> vtx=<M> lit=<K>
```

`N`은 요청한 양수 프레임 수, `M`은 마지막 측정 프레임의 ImGui 정점 수,
`K`는 해당 프레임에서 검은 clear 색과 다른 RGB 픽셀 수다.
픽셀은 렌더링 직후 버퍼 swap 전에 읽으며, alpha 값만 다른 경우는 그린 픽셀로 세지 않는다.

| 모드 | 시작·키 입력과 프레임 수 | 성공 판정 |
| --- | --- | --- |
| dev | 먼저 최소 1회 이상 숨김 프레임을 확인하고 실제 백틱으로 연다. 열리기 전 모든 프레임은 `vtx=0`, 열림 대기는 5초 이내다. `frames=N`은 열린 뒤 센 프레임 수이며 도중에 닫히면 실패한다. | 마지막 프레임 `M>0`, `K>0`. |
| 비dev | 첫 빈 프레임 뒤 백틱이 도착해도 UI가 생기지 않아야 한다. 키 도착을 확인한 뒤 N 프레임을 센다. 키가 5초 안에 오지 않으면 실패한다. | `vtx=0 lit=0`. |
| 비dev + `--play` | UI 시작·프로브와 5초 제한은 비dev와 같다. 재생은 첫 프레임부터 가능하다. | `vtx=0 lit>0`. |

네이티브는 첫 프레임 뒤에 SDL 이벤트 큐로 비반복 KeyDown, 반복 KeyDown, KeyUp을 한 번 주입한다.
웹은 [브라우저의 실제 키 경로](web.md#웹-스모크)를 사용한다. 비dev의 검은 픽셀 판정은
그리기를 잡아내지만, clear 호출 자체가 빠진 경우까지 보장하지는 않는다.

실패 시 출력 형식은 `rn-smoke: fail <이유>`이며, 결과 줄 끝에는 개행 문자 하나만 붙인다.
네이티브 앱과 검사 스크립트는 실패 시 0이 아닌 종료 코드를 반환한다.
웹 앱은 페이지에 결과를 기록하고, 브라우저 스크립트가 이 결과와 콘솔·네트워크 오류를 읽어
종료 코드로 변환한다. 전체 실행 로그에는 Cargo나 브라우저의 진단 메시지가 섞일 수 있으므로,
한 줄 출력 계약은 앱 자체의 결과에 적용하며 네이티브 테스트는 subprocess의 stdout 전체를 대조한다.

## 실행이 안 될 때

- SDL3 빌드 오류: C/C++ 도구와 창 시스템 개발 패키지가 설치되어 있는지 확인한다. 첫 소스 빌드가
  오래 걸리는 현상과 실제 빌드 실패를 구분한다.
- 창·GL 초기화 오류: 그래픽 세션과 OpenGL 3.3 드라이버를 확인한다.
  화면이 없는 headless 환경에서는 `scripts/smoke-native.sh`와 `--dev`로 검사한다.
- 개발 도구가 보이지 않음: `--features dev`로 빌드했는지 확인한 뒤 물리 백틱 키를 누른다.
  텍스트 입력 포커스를 해제하고 Ctrl·Alt·GUI 키를 뗀 상태에서 다시 시도한다.
- `dev tools did not open within 5s`: 실제 키 이벤트 도착 여부, scancode, 반복 입력 판정을 확인한다.
  `smoke key probe did not arrive within 5s`는 비dev 모드에서 키 프로브가 큐에 도착하지 않은 경우다.
- 라벨이 `?`: 한글 폰트를 포함하지 않으므로 ASCII 영어 라벨만 사용한다.
- dev 모드의 `vtx=0`·`lit=0`: UI 생성이나 렌더링 호출이 빠졌는지 확인한다. 반대로 비dev의 `lit>0`은
  검은 배경 외에 무언가 그려졌다는 뜻이다. 빌드 모드를 확인하고 판정 임계값을 낮추지 않는다.
- `lit=0: sprite playback drew nothing`: 재생 draw 호출과 PNG에 불투명하면서 검정이 아닌 픽셀이 있는지 확인한다.
- `no animation named ...`·`sprite sheet has no animations`: animation 이름을 확인하거나 편집기에서 프레임이 들어 있는 animation을 만든다.
- `unsafe image path: ...`·`image ... must share sidecar directory ...`: PNG와 사이드카를 같은 디렉터리에 두고 `image` 필드에는 파일 이름만 적는다.
- `unsupported sprites.json version ...`·JSON/범위 검증 오류: [형식 규칙](sprites.md#사이드카-형식)을 확인한다. 잘못된 메타데이터를 자동으로 지우고 열지 않는다.
- `sheet ... exceeds GL_MAX_TEXTURE_SIZE ...`: 이미지 크기가 GL 최대 텍스처 한계를 넘은 경우다. 시트를 작게 나누거나 더 큰 텍스처를 지원하는 환경에서 연다.
- `missing required tool`: 오류 메시지에 표시된 `Xvfb`, `python3`, `timeout`, `nm` 등의 도구를 설치한다.

Decided in: isk-kznl, isk-srq5, isk-35g9, isk-7w7b, isk-fcuz, isk-hk1c
