# Check raw path endings before normalized components

## What happened
- Sprite Library의 Load 활성 조건이 끝의 `/`만 검사한 뒤 Path::file_name을 사용해 `assets/.`와 `/tmp/a.png/.`를 파일 경로로 허용했다.
- isk-d69f.11t 리뷰 finding 1에서 발견했고, 디렉터리 구성 요소가 있는 입력을 거부하는 테스트로 고쳤다.

## Why
- 경로 구성 요소 API는 끝의 구분자와 `.`를 정규화한다. 정규화한 file_name이 있다는 사실만으로 원래 입력이 파일 이름으로 끝났다고 판단할 수 없다.

## Do this instead
- 입력 문자열의 끝 문법이 의미를 가지면 구성 요소 API를 쓰기 전에 마지막 원문 구성 요소가 빈 값, `.`, `..`인지 확인한다.
- 정규화 결과가 같은 파일 이름처럼 보이는 `a/`, `a/.`, `a/..`를 거부 사례에 포함하고 일반 파일 경로도 함께 검사한다.
- 파일 존재 여부 검사 없이 문법만 판단하는 UI에서는 이 구분을 헬퍼와 테스트에 명시한다.

Learned in: isk-d69f.st8
