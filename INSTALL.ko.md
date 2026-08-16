# TFM2 Atlas 1.0.33 설치 안내

## Dashboard만 설치

1. `Dashboard/mods/tfm2_atlas_core`와 `Dashboard/mods/tfm2_atlas_client_055`를 게임 설치 경로의 소문자 `mods` 폴더에 복사합니다.
2. 게임에서 두 코드 모드를 활성화하고 커리어를 시작합니다.
3. `TFM2.Atlas.Dashboard.exe`는 게임 밖 원하는 폴더에서 실행합니다. 필요한 DLL이 함께 배포된 경우에는 EXE 옆에 둡니다.

Dashboard는 Editor EXE나 `tfm2_atlas_editor` 없이 모든 Dashboard 기능과 티어 적용을 수행합니다.

## Editor도 설치

1. 위의 공통 두 모드에 `Editor/mods/tfm2_atlas_editor`를 추가합니다.
2. 공통 모드가 양쪽 폴더에 중복되어 있으면 동일한 해시의 파일이므로 게임 `mods`에는 각 모드를 한 번만 둡니다.
3. `TFM2.Atlas.Editor.exe`는 게임 밖 원하는 폴더에서 실행합니다.

Editor는 Core와 Editor 연결 상태를 따로 표시합니다. 세 모드 중 하나가 없으면 편집 화면 대신 설치 안내를 표시합니다.

구형 브리지나 티어 모드가 같은 포트를 사용하면 삭제하지 말고 게임 밖 백업 폴더로 옮겨 충돌을 방지하세요. EXE와 앱 폴더는 `mods`에 넣지 않습니다.
