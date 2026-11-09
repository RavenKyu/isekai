# 스프라이트 라이브러리

PNG 시트를 격자로 나누어 타일 메타데이터, 여러 타일로 된 sprite, 이름 붙은 clip을 만든다.
편집·영역 오려 내기·타일셋 추출은 네이티브 `dev`에서만 제공한다. 런타임 재생은 모든
빌드에서 가능하며 웹은 코드로 만든 예제의 재생만 제공한다. 명령은 checkout 최상단 기준이다.
처음 실행 환경은 [개발 시작](README.md)을 따른다.

## 사이드카 형식

`cave-01-wall.png`의 메타데이터는 같은 디렉터리의 `cave-01-wall.sprites.json`에 저장한다.
다음은 형식 v1의 전체 예시다. 이미지 파일 자체는 포함하지 않으며, 실제 실행에는 이 격자와
좌표를 수용하는 PNG가 필요하다. 저장소에서 바로 재생할 수 있는 자체 제작 fixture는
`tests/fixtures/example.png`와 `tests/fixtures/example.sprites.json`이다.

```json
{
  "version": 2,
  "image": "cave-01-wall.png",
  "grid": { "tile": 16, "offset": [0, 0], "spacing": 0 },
  "tiles": [
    {
      "col": 3, "row": 1,
      "name": "torch-a",
      "tags": ["light"],
      "props": { "glow": "1" },
      "animation": {
        "mode": "loop",
        "frames": [ { "col": 3, "row": 1, "ms": 120 }, { "col": 4, "row": 1, "ms": 120 } ]
      }
    }
  ],
  "sprites": [
    { "name": "door", "kind": "rect", "col": 0, "row": 2, "cols": 2, "rows": 3, "pivot": [16.0, 48.0] },
    { "name": "tree", "kind": "set",
      "parts": [ { "col": 5, "row": 0, "dx": 0, "dy": 0 }, { "col": 6, "row": 1, "dx": 16, "dy": 16, "flip_x": true, "rotation": 90 } ],
      "pivot": [8.0, 24.0] }
  ],
  "clips": [
    { "name": "door-open", "mode": "once", "tags": [], "props": {},
      "frames": [ { "sprite": "door", "ms": 100 } ] }
  ]
}
```

| 필드 | 의미 |
| --- | --- |
| `version` | 1과 2를 읽으며 항상 2로 저장한다. 다른 버전은 오류다. |
| `image` | 사이드카 디렉터리를 기준으로 한 이미지 상대 경로. |
| `grid` | 정사각 타일 한 변 `tile`, 왼쪽·위 여백 `offset: [x, y]`, 타일 사이 `spacing`(px). |
| `tiles` | 메타가 있는 셀만 저장한다. `(col,row)`, 선택적 `name`, 문자열 목록 `tags`, 문자열 값 맵 `props`, 선택적 `animation`. |
| `sprites` | 유일한 `name`, `rect` 또는 `set`, 경계 상자 왼쪽 위 기준 픽셀 `pivot`. |
| `part.flip_x` | 가로 반전 여부. 기본 false는 생략한다. |
| `part.rotation` | 화면 기준 시계 방향 회전(0, 90, 180, 270도). 기본 0은 생략한다. |
| `clips` | 유일한 `name`, `mode`, `tags`, `props`, `{sprite, ms}` 프레임 목록. 상태머신 대신 이름만 남긴다. |

`rect`는 시작 셀과 `cols × rows`이며, `set`은 셀마다 픽셀 오프셋 `dx,dy`를 둔다.
음수 오프셋도 허용한다. 두 종류 모두 셀 단위로 그려서 원본 시트의 spacing이 조합한
오브젝트의 틈으로 들어가지 않는다. set의 part는 가로 반전 후 회전한다. rect와 타일 애니메이션 프레임에는 변형이 없다.
v1의 part에 변형 필드가 명시되면 false나 0이라도 거부한다. null과 허용되지 않은 각도도 오류다.
타일 애니메이션은 sprite 이름 대신 `(col,row,ms)`를 쓴다.
모드는 `loop`(반복), `once`(끝에서 마지막 프레임 유지), `pingpong`(양 끝을 중복하지 않고 왕복)이다.
길이 1·2의 pingpong도 지원한다. 프레임의 `ms`는 1 이상이며 프레임 이벤트는 없다.

읽기는 입력 순서를 유지한다. 저장은 복사본의 타일을 `(row,col)`, sprite·clip을 이름 순으로
정렬하고 pretty JSON 뒤에 개행 하나를 붙인다. 속성 키는 정렬된다. 정규형은 다시 저장해도
바이트가 같다. 위 예시의 공백·줄 배치는 설명용 원문이며 첫 Save에서 정규형으로 바뀐다.

다음은 실패한다. 조용히 삭제하거나 고치지 않는다.

- 다른 버전, 모르는 필드, 잘못된 이미지 경로, `tile=0` 또는 격자 산술 오버플로.
- 중복 타일 좌표·빈 이름·중복 이름, 0 크기 rect, 비어 있는 set.
  타일 이름은 아예 생략할 수 있다.
- 빈 프레임 목록, `ms=0`, 없는 sprite를 가리키는 clip, 문자열이 아닌 속성 값.
- 이미지 범위 밖의 타일·sprite·타일 애니메이션 프레임, 유한하지 않은 pivot.

`store::parse/load`가 이미지 없는 검증을 하고, 이미지를 여는 쪽이 `validate_bounds`까지 한다.
Save는 같은 디렉터리의 새 임시 파일에 쓰고 `sync_all` 후 rename으로 교체한다. 실패 때
임시 파일을 정리하며 대상에는 이전 내용 또는 새 내용이 남는다. 없는 부모 디렉터리를
이 저장 함수가 만들지는 않는다. 추출의 기존 JSON 보호는 [추출 절](#영역-후보와-타일셋-추출)을 따른다.

## 경로 규칙

`image`는 비어 있지 않은 상대 경로다. 절대 경로, `..`, `.`, 백슬래시, 빈 구성 요소
(`a//b.png`나 끝의 `/`), 드라이브 접두어·콜론, NUL은 거부한다.
형식 자체는 `sub/x.png`도 허용하지만 네이티브 런타임은 추가로 이미지와 사이드카가 같은
디렉터리에 있어야 한다. 편집기·추출기는 항상 이미지 파일 이름 하나만 쓴다.
기존 이미지의 심볼릭 링크가 사이드카 디렉터리 밖을 가리키면 로더가 거부한다.
편집기도 PNG를 열 때 같은 규칙을 적용하고, 사람이 지정한 디렉터리 밖으로 가는 파일
심볼릭 링크를 `unsafe image path`로 거부한다. 같은 디렉터리 안을 가리키는 파일 링크와
이미지·부모가 같은 실제 디렉터리로 모이는 디렉터리 링크는 허용한다.

편집기로 PNG를 열 때 사이드카가 잘못됐거나 `image`가 연 PNG 이름과 다르면 기존 시트를
유지하고 오류를 표시한다. 실패한 사이드카를 빈 메타데이터로 취급해 덮어쓰지 않는다.

## 격자

`offset[0]`은 왼쪽, `offset[1]`은 위 여백이다. x와 y가 달라도 그대로 적용된다.
셀의 원점과 개수는 다음과 같다. 여백이 이미지 크기를 넘으면 해당 축의 셀 수는 0이다.

```text
x = offset.x + col * (tile + spacing)
y = offset.y + row * (tile + spacing)
cols = floor((width  - offset.x + spacing) / (tile + spacing))
rows = floor((height - offset.y + spacing) / (tile + spacing))
```

오른쪽·아래의 완전한 셀이 아닌 자투리는 버리고 `leftover ... px`로 알린다.
클릭은 여백·spacing·자투리에서 셀을 선택하지 않는다. tile 16, offset [0,0], spacing 0이면
256×64는 16×4, 256×144는 16×9, 480×800은 30×50칸이다.
UV는 셀의 픽셀 가장자리를 이미지 크기로 나눈 값이며 반 픽셀 보정 없이 nearest 필터와 정수 배율을 쓴다.

## 편집기 사용

```sh
cargo run --features dev
```

백틱을 눌러 메뉴를 열고 `Assets > Sprite Library`를 켠다. 창은 처음에는 닫혀 있다.
PNG마다 별도의 편집 창이 열린다. 같은 파일을 다시 열면 기존 창이 앞으로 온다.
Library 창을 닫아도 편집 창은 유지된다. 웹 dev에는 Assets 메뉴가 없다. `View`의 기존 개발 창은 그대로 쓸 수 있다.

1. `PNG path` 입력 후 `Load`, 파일 드래그 앤 드롭, `Browse...`의 네이티브 파일
   대화상자, `assets/` 목록 중 하나로 PNG를 연다. 드롭은 도구가 숨겨져 있어도 창을 연다.
   입력칸 오른쪽 `Load`는 빈 값·공백만 있거나 마지막 경로 구성 요소가 비어 있거나 `.`·`..`이면 꺼진다.
   따라서 `/`로 끝나는 경로도 꺼진다. 입력의 앞뒤 공백은 지우고 연다. 파일 존재 여부는 Load할 때 확인한다.
   `Browse...`와 `assets/` 목록은 선택 즉시 열며 Load 버튼이 필요 없다.
   디렉터리 밖을 가리키는 PNG 심볼릭 링크는 이미 열린 파일을 가리켜도 포커스 전에 거부한다.
   오류는 Library 상태 줄에 남고, 열린 창과 입력 경로 및 디스크는 유지된다.
   목록은 현재 checkout의 assets 아래를 재귀 탐색하며 숨김 항목·심볼릭 링크를 따르지 않는다.
   `Refresh assets/`로 다시 읽는다. 확장자 대소문자는 무시하고 PNG만 받는다.
2. 마젠타 등을 투명하게 만들려면 `Key color`를 켜고 `RGB hex`에 `ff00ff` 같은 값을
   넣은 뒤 Load한다. 정규화와 키 색 처리는 메모리 이미지에 적용한다. 원본 PNG를 바꾸지 않는다.
   `Reload image`도 현재 키 색 옵션으로 이미지를 다시 읽는다.
3. `Tile`, `Offset x`, `Offset y`, `Spacing`을 넣는다. 정수 음수는 0, tile의 0은 1로
   제한한다. 변경으로 기존 메타가 범위 밖이 되면 경고하고 격자를 되돌린다.
4. `Guess`는 반복 패턴의 후보를 제시하고 점수는 아래 로그에 남긴다. `Apply ... offset (x,y)`를 눌러야
   tile과 x·y 여백이 함께 들어가며 spacing은 유지된다. 자동 적용하지 않는다.
   고유 타일만 모은 시트에서는 추정 정확도가 떨어지므로 사람이 정한 값이 우선이다.
   Guess는 백그라운드에서 실행하며 검사한 offset 수에 따른 진행률을 표시한다.
   실행 중에도 격자를 편집하거나 다른 창을 쓸 수 있다. `Cancel Guess`로 취소할 수 있으며, 창을 닫아도
   작업이 취소된다. `Reload image`는 이전 이미지의 작업과 결과를 버린다.
5. `Show grid`, 배율 1–16(`Zoom` 또는 이미지 위 Ctrl+휠)을 조정한다.
   클릭은 한 셀, Shift+클릭은 앵커부터 사각 범위, Ctrl+클릭은 토글이다.
   이미지 안의 격자 밖을 클릭하면 선택을 비운다.
6. `Tiles`는 단일 선택의 이름·쉼표 구분 태그·한 줄씩 `key=value` 속성을 편집한다.
   `=`가 없는 속성은 오류다. `Tile animation`을 켜고 mode, col/row/ms, 프레임 추가·위아래·삭제를 편집한다.
   다른 셀을 프레임으로 넣으려면 `Pin tile`을 켠다. 켜 두면 시트에서 다른 셀을 골라도 편집 대상이 그대로이고,
   `Add selected cells`가 고른 셀들을 행 순서, 같은 행에서는 왼쪽부터 프레임에 더한다. 끄면 다시 단일 선택을 따른다.
   격자를 바꾸거나 시트를 다시 열면 고정이 풀린다.
7. `Sprites`의 `New rect from selection` 또는 `New set from selection`으로 오브젝트를 만든다.
   rect는 선택의 경계 사각, set은 선택 셀만 담는다. pivot 기본값은 경계 상자 중심이며 수정할 수 있다.
   rect 좌표·크기, set 각 part의 셀과 음수도 가능한 dx/dy, Flip X와 Rotation을 편집한다.
   `Suggest from objects`는 연결된 픽셀 덩어리를 셀 사각으로 제안하며 `Add`를 눌러야 sprite가 된다.
   이름 변경은 clip 참조도 갱신한다. clip이 참조하는 sprite는 삭제할 수 없다.
8. `Clips`에서 clip을 만들고 이름·mode·태그·속성·프레임(sprite와 ms)을 편집한다.
   새 clip은 빈 프레임 목록으로 시작하므로 최소 하나를 추가해야 Save된다.
9. `Preview`에서 clip, 타일 애니메이션, 정지 sprite를 고른다. `Play`/`Pause`, `Restart`,
   배율 1–8, 프레임 번호·경과 ms, pivot 십자를 확인한다. once는 마지막 프레임에 머문다.
10. 창 제목의 `*`는 변경 표시다. `Save`는 변경이 있을 때만 켜지고 JSON 저장에 성공하면
    다시 꺼진다. 사이드카가 없는 PNG는 열자마자 켜진다. `Reload image`는 현재 메타를
    유지하며 PNG만 다시 읽는다. 메타까지 다시 읽으려면 저장 후 편집 창의 닫기(x)로 닫고 PNG를 다시 Load한다.
    닫을 때 변경이 남아 있으면 `Discard`/`Cancel`로 확인한다. 앱 종료는 확인 없이 미저장 변경을 잃는다.

각 편집 창 아래에는 검은 배경·흰 글자의 로그 영역이 있다. Save·Reload·검증 결과와
Guess의 진행·점수가 시스템 현지 시각 `HH:MM:SS`와 함께 쌓인다. 최대 500줄을 유지한다.
스크롤해서 읽고 텍스트를 선택해 복사할 수 있다. 아래쪽에서 읽는 동안에는 새 줄을 따라가며,
위쪽을 읽고 있으면 스크롤 위치를 유지한다. `Clear`로 해당 창의 로그만 지운다.
Load 실패와 assets 스캔 오류는 Library의 상태 문구에 표시한다.

`Grid updated`, `Tile metadata updated`, `Sprite updated`,
`Sprite renamed and clip references updated`, `Clip updated` 중 같은 기록이 바로 이어지면
새 줄 대신 마지막 줄의 시각과 횟수 `(xN)`을 갱신한다. 오류·Save·Reload·Guess 등 다른 기록이나
`Clear`는 묶음을 끊는다. 묶음은 문구만 비교하므로 편집 대상이 바뀌어도 이어지며, 로그를 남기지 않는 선택 변경과 sprite·clip 삭제는 묶음을 끊지 않는다. 로그의 500줄 상한은 유지된다.

메모리 이미지는 `Save image as`로 PNG 경로를 정하기 전에는 Save·Reload가 비활성이다.
PNG 저장 후 `Save`로 메타를 따로 쓴다. 다른 편집 창이 열고 있는 PNG 경로에는 저장할 수 없다. GL 최대 텍스처 크기를 넘으면 치수와
`GL_MAX_TEXTURE_SIZE` 오류를 한 줄로 표시하고 기존 시트를 유지한다.
라벨은 ASCII 영어이고 한글 폰트·undo·배치 설정 저장은 제공하지 않는다.

## 영역 후보와 타일셋 추출

사람의 로컬 작업 흐름은 큰 시트 → 패널 후보 선택 → 맵 이미지 → 고유 타일셋이다.
큰 시트에서 후보를 직접 확인한 뒤 개별 또는 일괄 저장할 수 있다.

1. 큰 PNG를 열고 `Regions`에서 배경 `Auto`/`Transparent`/`Color`, hex, `Min size`를 정해
   `Find regions`를 누른다. Auto는 투명 픽셀이 있으면 알파를, 없으면 좌상단 색을 배경으로 쓴다.
   기본 Min size는 32이며 8방향 연결 덩어리의 bbox를 제안한다. 후보는 JSON에 저장하지 않는다.
2. 후보를 선택하고 `Crop to new image`를 누른다. 선택 영역은 다음 프레임에 새 창의
   메모리 이미지로 열리며, 원본 창의 변경은 유지된다.
   격자는 유지하고 타일·sprite·clip 메타는 빈 상태로 시작한다.
   또는 `Output dir`과 `Save all regions`로 후보 전체를 `<stem>-region-<n>.png`로 저장한다.
   기본 폴더는 `assets/splites/<stem>/`이며 번호는 1부터 목록 순이다. 대상 중 사이드카가
   하나라도 있거나, 출력 이름이 심볼릭 링크이거나, 어느 편집 창에든 열린 PNG면 디렉터리를
   만들기 전에 전체를 거부한다. 그 밖의 기존 PNG는 교체한다. 검사는 버튼을 누를 때 한 번 한다.
   열린 이미지가 충돌하면 `sidecar, symlink or open image conflicts: ...`에 경로가 표시된다.
   열린 PNG만 걸렸다면 그 편집 창을 닫거나 다른 Output dir을 고른다. 목록의 경로가 사이드카이거나 심볼릭 링크 이름이면 창을 닫아도 거부가 남는다.
   백그라운드 진행률과 `Cancel region save`를 표시한다. 취소는 파일 사이에서 확인하므로
   이미 쓴 파일은 남는다. `Reload image`와 창 닫기도 저장을 취소한다. 이때 쓰던 파일 하나는
   끝까지 쓰고, Reload는 그 파일까지 로그에 남긴 뒤 취소를 적는다. 그 뒤로는 새 이미지의 로그에
   이전 작업의 결과가 들어오지 않는다. 일괄 저장은 사이드카와 창을 만들지 않고 키 색도 적용하지 않는다.
3. `Image output`을 확인하고 `Save image as`로 crop PNG를 쓴다. 기본은
   `assets/splites/<crop 이름>/<crop 이름>.png`다. 이 버튼은 JSON을 만들지 않는다.
   그 경로에 PNG나 사이드카가 이미 있으면 쓰지 않고 거부한다. 심볼릭 링크 이름도 포함한다.
   PNG는 배타적으로 게시하므로 동시에 다른 쓰기가 만든 파일도 덮지 않는다.
   어느 편집 창에든 열린 PNG나 그 PNG를 가리키는 심볼릭 링크도 거부한다.
   `image ... is already open; choose another image path`가 표시되면 다른 Image output을 고른다. 창을 닫아도 그 PNG가 디스크에 남아 같은 경로는 계속 거부된다.
4. 배경을 제거하려면 그 PNG를 저장한 다음 `Key color`를 켜고 해당 창의 `Reload image`를 누른다.
   이어 격자를 직접 입력하거나 Guess/Apply로 정한다. crop과 키 색 제거는 이 두 단계다.
5. `Extract`의 `Analyze`를 누른다. 완전 투명 픽셀의 RGB를 0으로 정규화한 뒤
   기본적으로 바이트가 같은 타일만 묶고 첫 등장 순서를 유지한다. `Group rotations/flips`는
   기본 꺼짐이며 켜면 회전·반전 8종도 묶는다. 대칭 타일은 변형 없음이 우선이다.
   이 선택은 분석을 느리게 하며 변경하면 이전 분석이 무효화된다. 완전 투명 타일도 하나의 고유 타일이다.
   unique 수와 원래 격자 셀 영역의 `rebuild` 동일 여부를 확인한다. 자투리는 비교에서 제외한다.
   `Columns`(기본 16)와 `Output`을 정한다. 격자·이미지가 바뀌면 분석을 다시 해야 한다.
6. `Write tileset`으로 PNG와 빈 메타의 사이드카, 맵 배치 파일을 쓴다. 기본 출력은
   `assets/splites/<맵 stem>/<맵 stem>-tileset.png`다. tile은 유지하고 offset·spacing은 0이 된다.
   현재 창의 원본 PNG를 포함해 어느 편집 창에든 열린 PNG는 Output으로 쓸 수 없다.
   그 PNG를 가리키는 심볼릭 링크도 파일이나 디렉터리를 만들기 전에 거부한다.
   `image ... is already open; choose another image path`가 표시되면 다른 창이 연 PNG는 그 창을 닫거나 다른 Output을 고른다. 현재 창의 원본이면 다른 Output을 고른다.
   `Open result`로 결과를 바로 열 수 있다. sprite·clip은 자동 생성하지 않으므로
   런타임 재생 전 편집기에서 sprite와 최소 하나의 clip을 작성한다.

Find regions·Suggest from objects·Analyze는 백그라운드에서 실행되며 진행률과 Cancel을 표시한다. 실행 중 다른 입력과 창을 사용할 수 있다. Reload와 창 닫기는 작업을 취소하며, Analyze는 실행 중 격자나 묶기 설정이 바뀌거나 `Save image as`로 이름이 바뀌어도 취소되고 이전 결과를 버린다. Find regions 중 Background나 Min size를 바꾸면 진행 중인 검색은 취소된다.
출력은 `.png`만 받는다. checkout의 `assets/splites/` 안에서는 없는 디렉터리를 만들지만,
밖에서는 이미 존재하는 디렉터리만 쓸 수 있다. `..`와 심볼릭 링크를 거치는 출력 디렉터리는 거부한다.
저장소 안이면서 이 ignore 폴더 밖이면 커밋 대상이 될 수 있다는 상태 줄을 표시한다.

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

인자 순서는 `[--play <sidecar.json> [clip]] [--smoke-frames N]`이다. clip을 생략하면
이름 순 첫 clip을 고르며, 없으면 `sprite sheet has no clips`, 틀린 이름은 `no clip named ...` 오류다.
정수 배율은 `max(1, floor(viewport_height / 4 / sprite_height))`이고 pivot을 창 가운데에 둔다.
실패는 일반 실행에서 `rusty-nails: <이유>`, 스모크에서는 `rn-smoke: fail <이유>`다.

다음은 저장소에 포함되지 않은 사람의 로컬 파일 예다. 해당 PNG와 편집기로 만든 clip이
있는 사이드카를 이미 가지고 있을 때만 실행한다.

```sh
cargo run -- --play assets/splites/cave/cave-01-wall.sprites.json
```

## 웹 재생

```sh
scripts/build-web.sh
scripts/serve-web.sh
```

서버를 둔 채 `http://127.0.0.1:8080/?play=example` 또는
`http://127.0.0.1:8080/?play=example&clip=cycle`을 연다.
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

native 스모크는 fixture를 `target/` 아래에 복사해 재생 성공과 없는 파일·없는 clip·위험한
경로 실패를 함께 검사한다. 웹은 무인자/`--dev`/`--play` 셋 중 하나이며 인자를 조합하지 않는다.
비dev 재생은 `vtx=0 lit>0`이어야 성공이다. 원래 비dev 빈 화면은 `vtx=0 lit=0`, dev는
`vtx>0 lit>0`이다. 결과 한 줄과 시간 상한은 [스모크 출력 계약](development.md#스모크-출력-계약)을 따른다.

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
사람의 원본 `/home/raven/Works/splites/`와 결과 `assets/splites/`는 로컬 전용이다.
`/assets/splites/`는 gitignore에 있으며 이미지·결과 PNG·사이드카를 커밋하지 않는다.
라이선스는 이 문서에서 단정하지 않는다. 자동 검사·웹 예제는 코드가 만든 이미지로만 동작한다.

Decided in: rn-fcuz, rn-hk1c, rn-aw9h, rn-ut06, rn-6ptr, rn-8v66
