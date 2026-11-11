# 스프라이트 시트: Asset Library와 Sheet Editor

PNG 시트의 타일 메타데이터, sprite, animation을 편집한다. 편집·영역 오려 내기·타일셋 추출은 네이티브 `dev` 빌드에서만 제공한다. 런타임 재생은 모든 빌드에서 가능하며, 웹 빌드는 코드로 생성한 예제만 재생한다. 명령은 checkout 최상단 기준이다. 초기 환경 준비는 [개발 시작](README.md)을 따르며, [화면 시안](mockups/sheet-editor.html)도 참고할 수 있다.


## isekai 런타임 재생

시트 편집과 추출은 v0.6.0까지 기존 `cargo run --features dev`로 실행하며,
스프라이트 재생에는 새 `isekai` CLI를 쓸 수 있다.

```sh
cargo run -p isekai -- play tests/fixtures/example.sprites.json cycle
```

v1·v2·v3 사이드카 읽기와 안전한 이미지 경로 검증은 공통 `isekai-core`를 사용한다.
표시 배율은 `max(1, floor(viewport_height / 4 / sprite_height))`이며 pivot을 창 중앙에 맞춘다.
새 웹 경로는 `scripts/build-web-isekai.sh`와 `scripts/serve-web-isekai.sh`를 실행한 뒤
브라우저에서 `http://127.0.0.1:8080/?play=example&animation=cycle`을 연다. 기존 `--play` 플래그는
`rusty-nails`에 남고, 새 앱은 `play` 하위 명령을 사용한다.


## 용어

- **타일**은 시트를 격자로 나눈 한 칸이다. **타일셋**은 바닥·벽·물처럼 칸 단위로 배치하는 타일과 이름·태그·속성의 모음이며, `Tileset`에서 편집한다.
- **스프라이트**는 캐릭터·아이템·이펙트처럼 개별 단위로 다루는 그림 한 장으로, 하나 이상의 타일로 구성된다. `Sprites`에서 편집한다.
- **애니메이션**은 여러 스프라이트를 시간 순서대로 전환하는 프레임 묶음이다. 구버전 이름은 `clip`이며 `Animations`에서 편집한다. 타일 자체의 애니메이션은 지원하지 않는다.
- **Asset Library**는 `assets/` 아래의 PNG 파일을 찾아 여는 목록 창이고, **Sheet Editor**는 각 PNG 파일마다 열리는 편집 창이다. `Import`의 `Regions`와 `Extract tileset`은 각각 큰 시트에서 영역을 잘라 내거나 맵 이미지에서 고유 타일셋을 추출하는 전처리 도구다.
- PNG 파일 한 장과 사이드카 파일 하나가 타일셋·스프라이트·애니메이션 정보를 함께 담는다.

## 사이드카 형식

`cave-01-wall.png`의 메타데이터는 같은 디렉터리의 `cave-01-wall.sprites.json`에 저장한다. 아래 예시는 저장소 fixture인 `tests/fixtures/example.sprites.json`의 정규형 그대로이며, 실제로 재생하려면 같은 디렉터리에 `example.png`가 있어야 한다.

```json
{
  "version": 3,
  "image": "example.png",
  "grid": {
    "tile": 16,
    "offset": [
      0,
      0
    ],
    "spacing": 0
  },
  "next_id": 6,
  "tiles": [],
  "sprites": [
    {
      "id": 1,
      "name": "a",
      "kind": "rect",
      "col": 0,
      "row": 0,
      "cols": 1,
      "rows": 1,
      "pivot": [
        8.0,
        8.0
      ],
      "props": {}
    },
    {
      "id": 2,
      "name": "b",
      "kind": "rect",
      "col": 1,
      "row": 0,
      "cols": 1,
      "rows": 1,
      "pivot": [
        8.0,
        8.0
      ],
      "props": {}
    },
    {
      "id": 3,
      "name": "c",
      "kind": "rect",
      "col": 2,
      "row": 0,
      "cols": 1,
      "rows": 1,
      "pivot": [
        8.0,
        8.0
      ],
      "props": {}
    },
    {
      "id": 4,
      "name": "d",
      "kind": "rect",
      "col": 3,
      "row": 0,
      "cols": 1,
      "rows": 1,
      "pivot": [
        8.0,
        8.0
      ],
      "props": {}
    }
  ],
  "animations": [
    {
      "id": 5,
      "name": "cycle",
      "mode": "loop",
      "tags": [],
      "props": {},
      "frames": [
        {
          "sprite_id": 1,
          "ms": 150
        },
        {
          "sprite_id": 2,
          "ms": 150
        },
        {
          "sprite_id": 3,
          "ms": 150
        },
        {
          "sprite_id": 4,
          "ms": 150
        }
      ]
    }
  ]
}
```

| 필드 | 의미 |
| --- | --- |
| `version` | v1·v2·v3를 읽고 항상 v3로 저장한다. 다른 버전은 오류다. |
| `image` | 사이드카 디렉터리 기준 이미지 상대 경로. |
| `grid` | 정사각 타일 한 변 `tile`, 왼쪽·위 여백 `offset: [x, y]`, 타일 사이 `spacing`(px). |
| `next_id` | v3에서 다음에 부여할 양의 정수(1 이상 9223372036854775807 이하). 삭제해도 내려가지 않는다. |
| `tiles` | 메타가 있는 셀의 `(col,row)`, 선택적 `name`, `tags`, 문자열 맵 `props`. v3에는 타일 `animation`이 없다. |
| `sprites` | `id`, 유일한 `name`, `rect` 또는 `set`, 픽셀 `pivot`, 필수 문자열 맵 `props`, 선택적 `modified`. |
| `animations` | `id`, 유일한 `name`, `mode`, `tags`, `props`, `{sprite_id, ms}` 프레임과 선택적 `modified`. |
| `part.flip_x`, `part.rotation` | 가로 반전 후 화면 기준 시계 방향 회전(0·90·180·270도). 기본 false와 0은 생략한다. |

sprite와 animation은 한 시트 안에서 단일 ID 공간을 공유한다. ID는 1 이상 `next_id` 미만의 정수로 서로 중복되지 않는다. 이름을 바꿔도 ID와 프레임의 `sprite_id` 참조는 유지되며, 한 번 삭제한 ID는 다시 쓰지 않는다. `next_id`가 상한에 도달하면 새 항목 생성을 거부한다. 이 ID는 향후 사이드카 외부의 prefab이나 맵 파일에서 참조하기 위한 값이며, 외부 파일 형식은 아직 정의되지 않았다. `--play` 실행 시에는 animation 이름으로 대상을 지정한다.

`rect` 종류의 틀 크기는 `cols x rows`이며 별도 `canvas` 필드가 없다. 반면 `set`은 칸 단위 크기를 `canvas: [w, h]`에 저장한다. 틀은 오프셋 `(0,0)`에서 시작하며 모든 part가 틀 안에 들어와야 한다(v3에서는 음수 오프셋을 허용하지 않는다). `pivot`은 틀의 왼쪽 위 모서리를 기준으로 하며 빈 칸은 투명하게 처리한다. `set`의 각 part는 가로 반전을 먼저 적용한 뒤 회전하며, 시트의 `spacing`은 조합된 스프라이트 내부에 틈을 만들지 않는다. 편집기에서는 캔버스 가로·세로(W·H)를 각각 1~8칸 범위에서 생성·조정할 수 있으며, 새로 만든 sprite의 기본 `pivot`은 바닥 중앙인 `(w*tile/2, h*tile)`로 잡힌다. 기존 sprite의 `pivot`은 덮어쓰지 않고 그대로 유지한다.

v1·v2 사이드카를 읽을 때는 `clips`와 각 프레임의 sprite 이름을 읽어 들여, 먼저 sprite에 이름순으로 1부터 ID를 붙이고 이어서 animation에 이름순으로 ID를 매긴다. 구버전 `set`은 기존 경계의 왼쪽 위 원점이 `(0,0)`에 오도록 모든 part를 평행 이동하고, 경계 크기를 타일 칸 단위로 올림해 `canvas`를 부여한다(이때 `pivot` 수치는 바꾸지 않는다). v1의 part 변형 필드는 `false`나 `0`이 명시되어 있어도 거부하며, v2·v3는 허용된 각도(0·90·180·270)만 받는다. 구버전의 타일 `animation` 필드는 읽을 때 버리고, 버려진 셀 좌표를 편집기 로그와 네이티브 런타임 stderr에 경고로 남긴다. 편집기는 구버전 시트를 열자마자 변경 상태(`*`)로 표시해 곧바로 v3 형식으로 `Save`할 수 있게 한다.

v3 파일에 `clips`나 타일 `animation`이 있거나, 반대로 v1·v2 파일에 `animations`, ID·`next_id`·`sprite_id`, sprite의 `props`·`modified`, set의 `canvas`가 있으면 오류로 거부한다. v3에서는 ID·`next_id`·sprite `props`, set의 `canvas`, 프레임의 `sprite_id`가 모두 필수다. `modified`는 Unix epoch 초 단위 시각으로, 해당 항목이 실제로 편집될 때만 갱신하며 `Save`·`Load`·`Reload`나 단순 격자 변경만으로는 바뀌지 않는다. 화면에는 시스템 현지 시각으로 변환해 보여 준다.

애니메이션 재생 모드는 `loop`(반복), `once`(마지막 프레임에서 정지), `pingpong`(양 끝 프레임을 중복하지 않고 왕복) 세 가지이며, 각 프레임의 `ms`는 1 이상이어야 한다. 저장할 때는 타일을 `(row,col)` 순, sprite와 animation을 ID 순, 속성 맵(`props`)을 키 이름순으로 정렬한 뒤 pretty JSON 끝에 개행 문자 하나를 붙인다. 따라서 저장 → 읽기 → 재저장 결과가 바이트 단위로 일치한다.

지원하지 않는 버전이나 알 수 없는 필드, 잘못된 경로·격자 수치, 중복된 이름·좌표·ID, 0 이하이거나 `next_id` 이상인 ID, 비어 있는 `set`이나 `animation`, 존재하지 않는 `sprite_id` 참조, `0ms` 프레임, 틀 범위를 벗어난 part, 이미지 크기를 벗어난 타일·sprite, 유한하지 않은 `pivot` 값은 모두 거부한다. 편집 도중에는 임시로 빈 `set`이나 빈 `animation` 상태를 허용하지만, `Save`와 `Load` 시점에는 거부한다.

`store::parse`와 `store::load`가 이미지 없이 할 수 있는 검증을 먼저 수행하고, 이미지를 실제로 여는 단계에서 `validate_bounds`로 이미지 경계 검증까지 마친다.
`Save`는 대상과 같은 디렉터리에 새 임시 파일을 만들어 내용을 쓰고 `sync_all`을 호출한 뒤 `rename`으로 교체한다. 도중에 실패하면
임시 파일을 지우므로 대상 경로에는 이전 내용 또는 새 내용이 남는다. 존재하지 않는 부모 디렉터리를
저장 함수가 새로 만들지는 않는다. 타일셋 추출 시 기존 JSON을 보호하는 규칙은 [영역 후보와 타일셋 추출](#영역-후보와-타일셋-추출) 절을 따른다.
`sync_all`은 파일 데이터에 대해서만 호출하며 부모 디렉터리까지는 sync하지 않는다(이를 켜는 옵션도 없다).
따라서 갑작스러운 전원 중단이나 운영체제 크래시가 발생하면 `Save`가 성공을 반환했더라도 사이드카가 이전 내용으로
되돌아가거나 같은 디렉터리에 임시 파일이 남을 수 있다. 파일 이름 교체의 크래시 내구성은
Linux를 포함한 어느 플랫폼에서도 보장하지 않는다. 앞서 설명한 임시 파일 정리와 이전 내용 또는 새 내용은
저장이 오류를 돌려줄 때의 설명이며, 크래시나 강제 종료로 남은 임시 파일은 프로그램이 자동으로 지우거나
재시작 시 탐색하지 않는다.

## 경로 규칙

`image` 필드는 비어 있지 않은 상대 경로여야 한다. 절대 경로, `..`, `.`, 백슬래시(`\`), 빈 경로 요소
(`a//b.png`나 끝의 `/`), Windows 드라이브 접두어·콜론(`:`), NUL 문자는 모두 거부한다.
형식 문법 자체는 `sub/x.png` 같은 하위 경로도 허용하지만, 네이티브 런타임은 추가로 이미지와 사이드카가 같은
디렉터리에 있어야 한다고 요구하며, 편집기·추출기는 저장할 때 항상 이미지 파일 이름 하나만 쓴다.
기존 이미지의 심볼릭 링크가 사이드카 디렉터리 바깥을 가리키면 로더가 거부한다.
편집기에서 PNG를 열 때도 동일한 규칙을 적용해, 사람이 지정한 디렉터리 밖을 가리키는 파일
심볼릭 링크는 `unsafe image path` 오류로 거부한다. 같은 디렉터리 안의 파일을 가리키는 심볼릭 링크나,
이미지와 사이드카 부모가 결국 같은 실제 디렉터리로 해석되는 디렉터리 심볼릭 링크는 허용한다.

편집기에서 PNG를 열 때 사이드카가 잘못되었거나 `image`가 연 PNG 이름과 다르면, 기존에 열려 있던 시트 상태를
그대로 유지하고 오류를 표시한다. 잘못된 사이드카를 무시하고 빈 메타데이터로 덮어쓰지 않는다.

## 격자

`offset[0]`은 왼쪽 여백, `offset[1]`은 위쪽 여백이며 두 값이 서로 달라도 그대로 적용된다.
각 셀의 픽셀 원점 `(x, y)`와 전체 열·행 수 `(cols, rows)`는 다음 공식으로 계산한다. 여백이 이미지 크기를 넘으면 해당 축의 셀 수는 0이 된다.

```text
x = offset.x + col * (tile + spacing)
y = offset.y + row * (tile + spacing)
cols = floor((width  - offset.x + spacing) / (tile + spacing))
rows = floor((height - offset.y + spacing) / (tile + spacing))
```

오른쪽과 아래쪽 끝에 남는 불완전한 자투리 영역은 셀로 치지 않고 버리며, 화면에 `leftover ... px`로 알려 준다.
여백, `spacing` 틈, 자투리 영역을 클릭해도 셀이 선택되지 않는다. 예를 들어 `tile` 16, `offset` [0,0], `spacing` 0일 때
256×64 이미지는 16×4칸, 256×144는 16×9칸, 480×800은 30×50칸이 된다.
텍스처 UV는 셀의 픽셀 경계를 이미지 전체 크기로 나눈 값이며, 반 픽셀(half-pixel) 보정 없이 nearest 필터와 정수 배율로 그린다.

## 편집기 사용

```sh
cargo run --features dev
```

물리 백틱 키를 눌러 상단 메뉴바를 열고 `Assets > Asset Library`를 켠다(처음에는 닫혀 있다).
열린 PNG마다 독립된 `Sheet Editor` 창이 생기며, 이미 열린 파일을 다시 열면 해당 편집 창이 앞으로 올라온다.
`Asset Library` 창을 닫아도 개별 편집 창은 그대로 남는다. 웹 `dev` 빌드에는 `Assets` 메뉴가 없으며 `View`의 기존 개발 창은 그대로 쓸 수 있다.

1. `PNG path` 입력 후 `Load` 클릭, 파일 드래그 앤 드롭, `Browse...` 네이티브 파일
   대화상자, `assets/` 파일 목록 중 하나로 PNG를 연다. 드롭은 도구가 숨겨져 있어도 창을 연다.
   입력칸 옆 `Load` 버튼은 입력값이 비어 있거나 공백뿐일 때, 또는 경로의 마지막 요소가 비어 있거나(`/`로 끝나는 경우 포함) `.`·`..`일 때 비활성화된다.
   입력 문자열의 앞뒤 공백은 자동으로 제거한 뒤 열며, 실제 파일 존재 여부는 `Load` 시점에 확인한다.
   `Browse...`와 `assets/` 목록에서는 항목을 고르면 즉시 열리므로 `Load` 버튼을 따로 누를 필요가 없다.
   기준 디렉터리 밖을 가리키는 PNG 심볼릭 링크는 설령 이미 열려 있는 파일을 가리키더라도 창 포커스를 옮기기 전에 거부한다.
   오류가 나면 `Asset Library` 상태 줄에 메시지를 표시하고, 기존에 열린 창과 입력 경로 및 디스크 파일은 건드리지 않는다.
   `assets/` 목록은 현재 checkout의 `assets` 디렉터리 아래를 재귀적으로 탐색하되 숨김 항목과 심볼릭 링크는 건너뛴다.
   파일 목록을 갱신하려면 `Refresh assets/`를 누른다. 확장자는 대소문자를 구분하지 않고 `.png`만 표시한다.
2. 마젠타 같은 배경색을 투명하게 빼려면 `Key color`를 체크하고 `RGB hex`에 `ff00ff` 같은 값을
   입력한 뒤 `Load`한다. 정규화와 키 색 투명 처리는 메모리상의 이미지에만 적용되며 디스크의 원본 PNG 파일은 바꾸지 않는다.
   이미 열린 창에서 `Reload image`를 누를 때도 현재 설정된 키 색 옵션을 적용해 이미지를 다시 읽는다.
3. `Tile`, `Offset x`, `Offset y`, `Spacing` 값을 지정한다. 음수 입력은 0으로, `Tile`의 0은 1로
   자동 보정된다. 격자 값을 바꿨을 때 기존 타일·sprite 메타데이터가 새 격자 범위를 벗어나면 경고를 띄우고 격자 값을 이전 상태로 되돌린다.
4. `Guess`를 누르면 반복 패턴을 분석해 격자 후보를 제안하고 하단 로그에 점수를 기록한다. 제안된 후보 옆의 `Apply ... offset (x,y)` 버튼을 직접 눌러야만
   `tile`과 `x`·`y` 여백이 반영되며(이때 `spacing`은 기존 값 유지), 자동으로 덮어쓰지는 않는다.
   중복 없이 고유 타일만 모아 놓은 시트에서는 추정 정확도가 떨어지므로 사람이 직접 입력한 값을 우선한다.
   `Guess` 분석은 백그라운드 스레드에서 돌며 검사한 offset 수에 따른 진행률을 표시한다.
   분석 도중에도 격자 수치를 직접 수정하거나 다른 창을 조작할 수 있다. `Cancel Guess`로 취소할 수 있으며, 창을 닫아도
   작업이 취소된다. `Reload image`는 이전 이미지의 작업과 결과를 버린다.
5. `Show grid`, 배율 1–16(`Zoom` 또는 이미지 위 Ctrl+휠)을 조정한다.
   클릭은 한 셀, Shift+클릭은 앵커부터 사각 범위, Ctrl+클릭은 토글이다.
   이미지 안의 격자 밖을 클릭하면 선택을 비운다.
6. `Tileset`에서 한 타일의 이름, 쉼표 구분 태그, `Properties`의 key/value 행을 편집한다. 빈 행은 무시하고 빈 키나 중복 키는 오류다. 메타가 있는 칸에는 격자 표시와 무관하게 금색 표시가 나온다.
7. `Sprites` 목록에서 썸네일·크기·수정 시각을 보고 고른다. `New from selection`은 선택한 칸으로 set을 만들고 선택이 없으면 비활성이다. 선택 경계가 8 x 8보다 크면 거부한다. `+ New sprite`는 빈 2 x 2 틀을 만들며 타일을 하나 넣기 전에는 Save할 수 없다. 캔버스 W·H는 1~8이고 시트의 타일을 칸으로 끌어 놓는다. 칸의 `x` 또는 우클릭 `Clear`로 비우고 우클릭 메뉴에서 `Flip X`와 `Rotation`을 바꾼다. 크기를 줄이면 밖의 칸 수를 로그에 남긴다. 저장된 틀 덕분에 가장자리 빈 칸도 다시 열린다. 겹친 part, 격자에 맞지 않는 part 또는 8칸을 넘는 틀은 캔버스에서 읽기 전용이며 `Parts`에서 편집한다. 이름 변경은 ID 참조를 바꾸지 않고, 참조 중인 sprite는 삭제할 수 없다.
8. `Animations`에서 이름·mode·태그·속성과 sprite ID를 가리키는 프레임을 편집한다. 프레임마다 썸네일·sprite·ms와 `<`·`x`·`>`가 있고 현재 재생 프레임에 테두리가 표시된다. 새 animation은 프레임을 하나 이상 넣어야 Save할 수 있다.
9. 오른쪽 Preview는 항상 보인다. Tileset·Import는 선택 타일, Sprites는 선택 sprite와 pivot, Animations는 선택 animation을 표시한다. 선택이 256칸을 넘으면 이미지 대신 안내만 보인다. `Play`/`Pause`와 `Restart`는 Animations에서만 제공한다.
10. 창 제목의 `*`는 변경 표시다. `Save`는 변경이 있을 때만 켜지고 JSON 저장에 성공하면
    다시 꺼진다. 사이드카가 없는 PNG는 열자마자 켜진다. `Reload image`는 현재 메타를
    유지하며 PNG만 다시 읽는다. 메타까지 다시 읽으려면 저장 후 편집 창의 닫기(x)로 닫고 PNG를 다시 Load한다.
    닫을 때 변경이 남아 있으면 `Discard`/`Cancel`로 확인한다. 앱 종료는 확인 없이 미저장 변경을 잃는다.

각 편집 창 아래에는 검은 배경·흰 글자의 로그 영역이 있다. Save·Reload·검증 결과와
Guess의 진행·점수가 시스템 현지 시각 `HH:MM:SS`와 함께 쌓인다. 최대 500줄을 유지한다.
스크롤해서 읽고 텍스트를 선택해 복사할 수 있다. 아래쪽에서 읽는 동안에는 새 줄을 따라가며,
위쪽을 읽고 있으면 스크롤 위치를 유지한다. `Clear`로 해당 창의 로그만 지운다.
Load 실패와 assets 스캔 오류는 Asset Library의 상태 문구에 표시한다.

`Grid updated`, `Tile metadata updated`, `Sprite updated`,
`Sprite renamed`, `Animation updated` 중 같은 기록이 바로 이어지면
새 줄 대신 마지막 줄의 시각과 횟수 `(xN)`을 갱신한다. 오류·Save·Reload·Guess 등 다른 기록이나
`Clear`는 묶음을 끊는다. 묶음은 문구만 비교하므로 편집 대상이 바뀌어도 이어지며, 로그를 남기지 않는 선택 변경과 sprite·animation 삭제는 묶음을 끊지 않는다. 로그의 500줄 상한은 유지된다.

메모리 이미지는 `Save image as`로 PNG 경로를 정하기 전에는 Save·Reload가 비활성이다.
PNG 저장 후 `Save`로 메타를 따로 쓴다. 다른 편집 창이 열고 있는 PNG 경로에는 저장할 수 없다. GL 최대 텍스처 크기를 넘으면 치수와
`GL_MAX_TEXTURE_SIZE` 오류를 한 줄로 표시하고 기존 시트를 유지한다.
라벨은 ASCII 영어이고 한글 폰트·undo·배치 설정 저장은 제공하지 않는다.

## 영역 후보와 타일셋 추출

사람의 로컬 작업 흐름은 큰 시트 → 패널 후보 선택 → 맵 이미지 → 고유 타일셋이다.
큰 시트에서 후보를 직접 확인한 뒤 개별 또는 일괄 저장할 수 있다. Sheet Editor의 `Import` 탭에서 `Regions` 또는 `Extract tileset` 머리를 연다.

1. 큰 PNG를 열고 `Regions`에서 배경 `Auto`/`Transparent`/`Color`, hex, `Min size`를 정해
   `Find regions`를 누른다. Auto는 투명 픽셀이 있으면 알파를, 없으면 좌상단 색을 배경으로 쓴다.
   기본 Min size는 32이며 8방향 연결 덩어리의 bbox를 제안한다. 후보는 JSON에 저장하지 않는다.
2. 후보를 선택하고 `Crop to new image`를 누른다. 선택 영역은 다음 프레임에 새 창의
   메모리 이미지로 열리며, 원본 창의 변경은 유지된다.
   격자는 유지하고 타일·sprite·animation 메타는 빈 상태로 시작한다.
   또는 `Output dir`과 `Save all regions`로 후보 전체를 `<stem>-region-<n>.png`로 저장한다.
   기본 폴더는 `assets/sprites/<stem>/`이며 번호는 1부터 목록 순이다. 대상 중 사이드카가
   하나라도 있거나, 출력 이름이 심볼릭 링크이거나, 어느 편집 창에든 열린 PNG면 디렉터리를
   만들기 전에 전체를 거부한다. 그 밖의 기존 PNG는 교체한다. 검사는 버튼을 누를 때 한 번 한다.
   열린 이미지가 충돌하면 `sidecar, symlink or open image conflicts: ...`에 경로가 표시된다.
   열린 PNG만 걸렸다면 그 편집 창을 닫거나 다른 Output dir을 고른다. 목록의 경로가 사이드카이거나 심볼릭 링크 이름이면 창을 닫아도 거부가 남는다.
   백그라운드 진행률과 `Cancel region save`를 표시한다. 취소는 파일 사이에서 확인하므로
   이미 쓴 파일은 남는다. `Reload image`와 창 닫기도 저장을 취소한다. 이때 쓰던 파일 하나는
   끝까지 쓰고, Reload는 그 파일까지 로그에 남긴 뒤 취소를 적는다. 그 뒤로는 새 이미지의 로그에
   이전 작업의 결과가 들어오지 않는다. 일괄 저장은 사이드카와 창을 만들지 않고 키 색도 적용하지 않는다.
3. `Image output`을 확인하고 `Save image as`로 crop PNG를 쓴다. 기본은
   `assets/sprites/<crop 이름>/<crop 이름>.png`다. 이 버튼은 JSON을 만들지 않는다.
   그 경로에 PNG나 사이드카가 이미 있으면 쓰지 않고 거부한다. 심볼릭 링크 이름도 포함한다.
   PNG는 배타적으로 게시하므로 동시에 다른 쓰기가 만든 파일도 덮지 않는다.
   어느 편집 창에든 열린 PNG나 그 PNG를 가리키는 심볼릭 링크도 거부한다.
   `image ... is already open; choose another image path`가 표시되면 다른 Image output을 고른다. 창을 닫아도 그 PNG가 디스크에 남아 같은 경로는 계속 거부된다.
4. 배경을 제거하려면 그 PNG를 저장한 다음 `Key color`를 켜고 해당 창의 `Reload image`를 누른다.
   이어 격자를 직접 입력하거나 Guess/Apply로 정한다. crop과 키 색 제거는 이 두 단계다.
5. `Import`의 `Extract tileset`에서 `Analyze`를 누른다. 완전 투명 픽셀의 RGB를 0으로 정규화한 뒤
   기본적으로 바이트가 같은 타일만 묶고 첫 등장 순서를 유지한다. `Group rotations/flips`는
   기본 꺼짐이며 켜면 회전·반전 8종도 묶는다. 대칭 타일은 변형 없음이 우선이다.
   이 선택은 분석을 느리게 하며 변경하면 이전 분석이 무효화된다. 완전 투명 타일도 하나의 고유 타일이다.
   unique 수와 원래 격자 셀 영역의 `rebuild` 동일 여부를 확인한다. 자투리는 비교에서 제외한다.
   `Columns`(기본 16)와 `Output`을 정한다. 격자·이미지가 바뀌면 분석을 다시 해야 한다.
6. `Write tileset`으로 PNG와 빈 메타의 사이드카, 맵 배치 파일을 쓴다. 기본 출력은
   `assets/sprites/<맵 stem>/<맵 stem>-tileset.png`다. tile은 유지하고 offset·spacing은 0이 된다.
   현재 창의 원본 PNG를 포함해 어느 편집 창에든 열린 PNG는 Output으로 쓸 수 없다.
   그 PNG를 가리키는 심볼릭 링크도 파일이나 디렉터리를 만들기 전에 거부한다.
   `image ... is already open; choose another image path`가 표시되면 다른 창이 연 PNG는 그 창을 닫거나 다른 Output을 고른다. 현재 창의 원본이면 다른 Output을 고른다.
   `Open result`로 결과를 바로 열 수 있다. sprite·animation은 자동 생성하지 않으므로
   런타임 재생 전 편집기에서 sprite와 최소 하나의 animation을 작성한다.

Find regions·Analyze는 백그라운드에서 실행되며 진행률과 Cancel을 표시한다. 실행 중 다른 입력과 창을 사용할 수 있다. Reload와 창 닫기는 작업을 취소하며, Analyze는 실행 중 격자나 묶기 설정이 바뀌거나 `Save image as`로 이름이 바뀌어도 취소되고 이전 결과를 버린다. Find regions 중 Background나 Min size를 바꾸면 진행 중인 검색은 취소된다.
출력은 `.png`만 받는다. checkout의 `assets/sprites/` 안에서는 없는 디렉터리를 만들지만,
밖에서는 이미 존재하는 디렉터리만 쓸 수 있다. `..`와 심볼릭 링크를 거치는 출력 디렉터리는 거부한다.
저장소 안이면서 이 ignore 폴더 밖이면 커밋 대상이 될 수 있다는 상태 줄을 표시한다.

이전 기본 출력 폴더 `assets/splites/`가 있으면 Asset Library 상태 줄이 이동을 안내한다. 자동으로 이동하거나 삭제하지 않는다. checkout 최상단에서 새 폴더가 없을 때만 다음 명령으로 옮길 수 있다.

```sh
test -e assets/sprites || mv assets/splites assets/sprites
```

`assets/sprites`가 이미 있으면 아무 작업도 하지 않으므로 두 폴더를 사람이 직접 합친다. 사이드카 `image`와 맵 `tileset`은 파일 이름을 가리켜 폴더째 이동해도 유지된다. 이전 폴더에 출력할 때는 실제로 ignore되어 있어도 상태 줄에 `not ignored`가 붙을 수 있다.

`Write tileset`의 PNG는 재생성 결과로서 원자적으로 교체한다. JSON은 기존 파일이나 동시에 다른 쓰기가 만든
파일을 덮지 않도록 배타적으로 게시한다. 맵 파일은 PNG처럼 원자적으로 교체하되 심볼릭 링크인 맵 이름은 거부한다.
세 파일은 하나의 트랜잭션이 아니며 사이드카가 이미 있어도 맵은 갱신한다.
배타 게시에는 하드 링크를 지원하는 파일시스템이 필요하다. FAT/exFAT나 하드 링크를 지원하지 않는
일부 네트워크 마운트에서는 `Save image as`와 타일셋 JSON 게시가 IO 오류로 실패한다.
`Save all regions`와 `Write tileset`의 PNG·맵은 rename으로 교체하므로 하드 링크가 필요 없다.
`Write tileset`은 이때도 PNG를 이미 교체했고 `PNG written: ...; sidecar not written: sprite file IO: ...`를 표시한다.
하드 링크 생성이 실패하면 게시하려던 새 대상 파일은 생기지 않고 임시 파일은 지워진다.
출력 디렉터리를 새로 만들었다면 그 디렉터리는 남는다.
이때는 하드 링크를 지원하는 저장 위치를 사용한다. 다른 방식으로 자동 전환하지 않는다.
기존 사이드카 때문에 JSON 저장이 거부돼도 PNG는 이미 새 내용일 수 있다.
`PNG written: ...; sidecar not written: sidecar exists; remove it first`가 그 상태다.
이 경우 기존 JSON과 새 PNG의 격자가 맞는지 확인하고, 다른 출력 이름을 사용하거나
필요한 메타를 보존한 뒤 다시 시도한다. JSON을 무조건 지우지는 않는다.

파일 내용은 `sync_all`을 거치지만 부모 디렉터리는 sync하지 않고 이를 켜는 설정도 없다.
전원 중단이나 운영체제 크래시 뒤에는 `Save image as`와 타일셋 JSON처럼 배타 게시한 이름이 없을 수 있고,
rename으로 교체한 PNG·맵은 이전 상태로 남아 있을 수 있으며, 출력 디렉터리에 임시 파일이 남을 수 있다.
게시된 이름의 크래시 내구성은 Linux를 포함한 모든 플랫폼에서 보장하지 않는다. 앞 문단의 임시 파일
정리는 게시가 오류를 돌려줄 때의 동작이다. 크래시나 강제 종료로 남은 임시 파일은 프로그램이 지우지 않으며
다시 시작할 때 찾지도 않는다.

맵 파일은 타일셋과 같은 디렉터리의 `<원본 stem>.map.json`이다. 원본의 offset·spacing과
자투리는 저장하지 않고 완전한 셀을 행 우선으로 담는다. 타일셋 이름은 파일 이름 하나다.

| 맵 필드 | 의미 |
| --- | --- |
| `version` | 맵 형식 버전 1. |
| `tileset` | 타일셋 PNG 파일 이름. |
| `tile` | 정사각 타일의 한 변. |
| `cols`, `rows` | 원본에서 완전한 격자의 열과 행 수. |
| `cells` | 행 우선 셀 목록. `index`는 타일셋 번호, `flip_x`와 `rotation`은 선택적 변형이다. |

기본 변형은 생략한다. 예를 들어 두 셀의 정규형은 다음과 같다.

```json
{
  "version": 1,
  "tileset": "map-tileset.png",
  "tile": 16,
  "cols": 2,
  "rows": 1,
  "cells": [
    {
      "index": 0
    },
    {
      "index": 0,
      "flip_x": true,
      "rotation": 90
    }
  ]
}
```

## 런타임 재생

다음은 저장소의 코드 생성 fixture로 실행된다. 일반 실행은 창을 닫을 때까지 계속된다.

```sh
cargo run -- --play tests/fixtures/example.sprites.json
cargo run -- --play tests/fixtures/example.sprites.json cycle
cargo run -- --play tests/fixtures/example-turned.sprites.json
```

인자 순서는 `[--play <sidecar.json> [animation]] [--smoke-frames N]`이다. animation을 생략하면
이름 순 첫 animation을 고르며, 없으면 `sprite sheet has no animations`, 틀린 이름은 `no animation named ...` 오류다. v1·v2 사이드카도 재생하며 버린 타일 animation은 stderr로 경고한다.
정수 배율은 `max(1, floor(viewport_height / 4 / sprite_height))`이고 pivot을 창 가운데에 둔다.
실패는 일반 실행에서 `rusty-nails: <이유>`, 스모크에서는 `rn-smoke: fail <이유>`다.

다음은 저장소에 포함되지 않은 사람의 로컬 파일 예다. 해당 PNG와 편집기로 만든 animation이
있는 사이드카를 이미 가지고 있을 때만 실행한다.

```sh
cargo run -- --play assets/sprites/cave/cave-01-wall.sprites.json
```

## 웹 재생

```sh
scripts/build-web.sh
scripts/serve-web.sh
```

서버를 둔 채 `http://127.0.0.1:8080/?play=example` 또는
`http://127.0.0.1:8080/?play=example&animation=cycle`을 연다. 이전 링크의 `clip=`도 별칭으로 받는다.
`src/sprite/embedded.rs`의 표에는 `sprite::example()`이 만드는 64×16, 16px 네 색 시트만 있다.
파일 preload나 사람 이미지 임베드는 없으며 새 예제도 코드로 그린 것만 추가한다.
웹의 `play`는 임베드 이름, 네이티브의 `--play`는 사이드카 경로다.
틀린 웹 이름은 `no embedded sprite sheet named ...` 오류다. 자세한 실행·증거 위치는
[웹 안내](web.md#웹-스모크)를 따른다.

## 검사

```sh
cargo test --locked
cargo test --locked --features dev
scripts/smoke-native.sh
scripts/smoke-native.sh --dev
scripts/check-no-imgui.sh
scripts/build-web.sh
scripts/smoke-web.sh
scripts/smoke-web.sh --play
scripts/build-web.sh --dev
scripts/smoke-web.sh --dev
```

native 스모크는 fixture를 `target/` 아래에 복사해 재생 성공과 없는 파일·없는 animation·위험한
경로 실패를 함께 검사한다. 웹은 무인자/`--dev`/`--play` 셋 중 하나이며 인자를 조합하지 않는다.
비dev 재생은 `vtx=0 lit>0`이어야 성공이다. 원래 비dev 빈 화면은 `vtx=0 lit=0`, dev는
`vtx>0 lit>0`이다. 결과 한 줄과 시간 상한은 [rn-smoke 출력 계약](development.md#rn-smoke-출력-계약)을 따른다.

자동 테스트·CI는 사람 이미지를 읽지 않는다. 다음은 사람이 로컬 파일을 명시적으로
확인할 때만 쓰는 ignored 검사다. 파일이 없으면 건너뛴다.

```sh
RN_LOCAL_ASSETS=1 cargo test --locked local_assets_report -- --ignored --nocapture
```

텍스처 크기·필터·Drop 삭제와 외부 ID 해제는 실제 GL 검사로 본다.
`transformed_pixels_match_gpu_readback`은 코드로 만든 비대칭 4×4 타일의 여덟 변형을
`SpriteQuad`로 그려 GPU에서 읽은 픽셀을 `RgbaImage::transformed`와 비교하고,
90도 회전과 반전 후 90도 회전은 수기 기대값과도 비교한다. 방향 규약은 [사이드카 형식](#사이드카-형식)을 따른다.
이 검사는 디스플레이가 필요해 일반 테스트에서 무시되며 `scripts/smoke-native.sh`와
`scripts/smoke-native.sh --dev`에서 실행된다. `Playback::draw`의 dest·pivot 계산,
`Grid::uv`의 부분 시트 호출과 웹 ES 300의 실제 픽셀 방향은 측정하지 않는다.
App 필드 선언 순서와 전체 shutdown 순서가 잘못되는 변이를 자동으로 잡는 검사는 아직 없다. 리뷰에서 컨텍스트가
current인지, 텍스처·shader가 GLContext보다 먼저 해제되는지 코드를 확인해야 한다.

## 스크립트에서 가져온 범위

| `tile_extractor.py` | 현재 CPU API/사용처 |
| --- | --- |
| `normalize_alpha` | `RgbaImage::normalize_alpha`, 투명 RGB 정규화. |
| `apply_key_color` | `RgbaImage::apply_key_color`, 정확한 RGB 키. |
| `slice_grid` | `Grid::cells/origin` + `RgbaImage::tile`, offset x·y와 spacing 포함. |
| `find_offset`, `guess_tile_size` | `raster::guess_grid`, 적용 전 후보·점수만 반환. |
| `dedupe`, `count_unique`, `--transform` | `raster::dedupe_with`, 선택적 8변형과 unique·map·counts. |
| `map.json` | `sprite::tilemap`, 셀별 타일 번호·변형. |
| `build_sheet` | `raster::build_sheet`, Columns에 맞춰 PNG용 RGBA 이미지 생성. |
| `rebuild` | `raster::rebuild_matches`, 원래 격자 셀 영역 복원 비교. |
| `extract_objects` | `raster::find_objects`, `ObjectBox::snap`, Regions 후보와 Save all regions. |

기반 이미지 처리는 `src/raster.rs`, 분석은 `src/raster/analysis.rs`에 있다.
map.csv, 맵 에디터·런타임 타일맵, 레이어 자동 분류, 개별 tiles PNG, preview.png, JPEG, CLI 추출은
이번 범위 밖이다. 남겨 둔 제안은 트래커 백로그에서 다루며 이 화면에서 지원하지 않는다.

## 한계와 로컬 자산

PNG만 지원하며 undo, 프레임 이벤트, 상태머신·타일맵 배치는 지원하지 않는다.
앱 종료 시 미저장 변경 유실도 현재 한계다.
사람의 원본 `/home/raven/Works/splites/`와 결과 `assets/sprites/`는 로컬 전용이다.
`/assets/sprites/`와 이전 `/assets/splites/`는 gitignore에 있으며 이미지·결과 PNG·사이드카를 커밋하지 않는다.
라이선스는 이 문서에서 단정하지 않는다. 자동 검사·웹 예제는 코드가 만든 이미지로만 동작한다.

캔버스에서 8칸 넘는 이전 set은 읽기 전용이다. 타일 배수가 아닌 옛 set은 틀의 크기를 올림하면서 런타임 배율이 달라질 수 있다. 실제 FPS와 수백 프레임 띠의 비용은 측정하지 않았다.

Decided in: isk-srq5, isk-79ss, isk-aqpa, isk-fcuz, isk-hk1c, isk-aw9h, isk-ut06, isk-6ptr, isk-8v66, isk-y594, isk-utmw
