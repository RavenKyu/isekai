# ImGui 테스트의 모든 진입을 자식 프로세스로 격리한다

## What happened
- Save image as의 기본 테스트 wrapper는 자식 프로세스에서 ImGui 검사를 실행했다.
- dev 전체 스모크가 ignored 검사 본문을 직접 실행하면 다른 테스트와 같은 프로세스에서 컨텍스트를 만들게 되어 ContextThreadConflict로 실패했다.

## Why
- 기본 cargo test 경로만 보고 프로세스 격리를 wrapper에만 뒀다. --ignored로 직접 들어오는 검증 입구를 놓쳤다.

## Do this instead
- ImGui 검사 본문을 만들기 전에 자식 실행 여부를 확인한다. 기본 wrapper와 --ignored 직접 실행이 모두 그 경계를 지나게 한다.
- 부모는 정확한 테스트 경로로 자식을 실행하고 종료 성공뿐 아니라 실제 한 테스트가 실행됐는지도 확인한다.
- 기본 dev 테스트와 dev 전체 네이티브 스모크를 모두 실행한다. 한쪽의 성공으로 다른 진입 경로의 격리를 대신하지 않는다.

Learned in: isk-aw9h.cac
