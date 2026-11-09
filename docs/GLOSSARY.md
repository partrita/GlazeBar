# 용어 사전 (Glossary) — GlazeBar

| 용어 | 정의 | 관련 소스 / 컴포넌트 |
| :--- | :--- | :--- |
| **불투명 / 투명 모드 (Opacity Mode)** | 상단바의 배경 투명도 상태. 기존 6단계 퍼센트 방식에서 2단계(`불투명: 100%`, `투명: 0% + Colorkey`)로 단순화됨. | `settings_ui.rs`, `native.rs` |
| **Colorkey 투명화** | Windows `SetLayeredWindowAttributes`의 `LWA_COLORKEY`를 활용하여 윈도우 배경색(`palette.bg`)과 일치하는 픽셀을 완전 투명화하는 기술. | `native.rs` (`apply_window_opacity`) |
| **테두리 전용 작업공간 (Border-only Workspace)** | 작업공간 버튼의 배경 면 채우기(`ui.surface`, `ui.selected`)를 제거하고 테두리(Stroke)선으로만 박스 영역을 렌더링하여 투명한 바탕화면이 비치도록 하는 UI 표현 방식. | `views.rs` (`bar_with_settings`) |
| **상단바 높이 (Bar Height)** | 모니터 상단에 고정되는 GlazeBar의 기준 세로 크기(DPI 배율 적용 전 논리 픽셀 기준, 기본 40px). | `BarSettings`, `runtime.rs`, `views.rs` |
| **AppBar (Application Desktop Toolbar)** | Windows 쉘(`SHAppBarMessage`)에 등록하여 데스크톱 화면 상단에 예약 영역을 생성하고 창들이 겹치지 않도록 GlazeWM 및 타 윈도우의 작업영역(WorkArea)을 축소/조정하는 시스템 바. | `runtime.rs` (`Bar::position`, `appbar`) |
| **원활한 업데이트 (Seamless In-place Update)** | 기존 버전이 설치되어 있을 때 이전 인스톨러/파일을 자동으로 정리하고, 실행 중인 프로세스 종료 및 설정 백업/마이그레이션을 거쳐 새 버전을 즉시 교체 설치하는 프로세스. | `GlazeBar-Setup.iss`, `Install.ps1`, `Uninstall.ps1` |
| **이전 버전 상태 매니페스트 (`community-install.json`)** | 기존 설치된 파일 목록, 백업 위치, 시작프로그램 바로가기 경로 등을 기록해 둔 상태 파일. 업데이트 시 이전 파일 추적 및 안전 제거에 사용됨. | `Install.ps1`, `Uninstall.ps1` |
