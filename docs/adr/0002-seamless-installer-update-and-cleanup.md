# ADR 0002: 기존 버전 자동 제거 및 원활한(Seamless) 인스톨러 업데이트 파이프라인

- **상태 (Status)**: 승인됨 (Accepted)
- **일자 (Date)**: 2026-11-09
- **작성자**: Antigravity (grill-with-docs 세션)

---

## 1. 배경 및 맥락 (Context)
현재 GlazeBar + GlazeWM의 설치 방식은 Inno Setup 기반 GUI 설치 파일(`GlazeBar-GlazeWM-Setup-*.exe`)과 커뮤니티 번들의 `Install.ps1` 스크립트를 사용합니다.
그러나 기존 버전이 설치되어 있는 상태에서 새 버전을 업데이트할 때 다음과 같은 여러 병목 및 실패 요인이 존재하여 사용자가 업데이트에 큰 불편을 겪고 있습니다:
1. **실행 중인 프로세스 충돌**: GlazeBar와 GlazeWM은 백그라운드 상주 프로그램이므로 업데이트 실행 시 `Get-Process` 검사에서 에러(`throw 'Exit GlazeWM and GlazeBar before installing...'`)가 발생하여 설치가 즉시 중단됨.
2. **이전 Inno Setup 제거 미수행**: 새 버전 Inno Setup 인스톨러 실행 시 기존 버전의 언인스톨러(`UninstallString`)가 자동으로 호출되지 않아 레지스트리 및 프로그램 폴더 내 불일치 발생.
3. **시작프로그램 바로가기 충돌**: `Install.ps1`에서 시작프로그램 바로가기가 이미 존재하면 `throw 'Startup shortcut already exists; it was not overwritten.'` 예외가 발생하여 업데이트가 중단됨.
4. **사용자 설정(`config.yaml`, 테마 설정) 보존 여부 모호**: 단순 덮어쓰기나 롤백 시 사용자가 커스텀 설정한 단축키, 상단바 옵션이 유실될 위험.

## 2. 의사결정 동인 (Decision Drivers)
- **원클릭 업데이트 편의성**: 사용자가 작업 관리자나 명령줄에서 수동으로 프로세스를 종료하지 않아도 설치 프로그램이 알아서 안전하게 종료 및 갱신을 진행해야 함.
- **클린 업데이트 (Clean Reinstallation)**: 이전 버전의 잔여 파일이나 구버전 언인스톨러를 깔끔하게 제거하고 새 버전을 설치해야 함.
- **사용자 커스텀 설정 보존**: 업데이트 시 사용자의 개인화 설정(`config.yaml`, `theme-state/glazebar-settings.json`)이 초기화되지 않고 안전하게 보존/마이그레이션되어야 함.
- **백신 오탐 방지 규칙 준수**: `AGENT.md`에 명시된 비인가 API 금지, `-ExecutionPolicy Bypass` 금지, 런타임 다운로더(Dropper) 패턴 금지 수칙을 100% 준수해야 함.

## 3. 검토된 대안들 (Considered Options)
1. **프로세스 처리 방식**:
   - 대안 A: 기존처럼 에러를 던지고 사용자 수동 종료 강제 (사용자 경험 저하)
   - 대안 B (채택안): 업데이트 시작 시 `Stop-Process`로 실행 중인 `glazewm`, `GlazeBar`, `WorkspaceToggle`을 안전하게 자동 종료
2. **구버전 언인스톨**:
   - 대안 A: 파일만 덮어쓰기 (구버전 레지스트리 및 삭제된 파일 찌꺼기 잔존)
   - 대안 B (채택안): Inno Setup 레지스트리 및 `community-install.json`을 조회하여 이전 버전 언인스톨러를 사일런트 모드로 선행 실행 후 클린 재설치
3. **설정 파일 보존**:
   - 대안 A: 완전 초기화 (사용자 단축키 및 상단바 설정 유실)
   - 대안 B (채택안): 업데이트 전 임시 마이그레이션 백업에 `config.yaml`과 `theme-state`를 보존한 뒤 새 설치 완료 시 자동 복원
4. **시작프로그램 바로가기**:
   - 대안 A: 충돌 시 에러 발생
   - 대안 B (채택안): 기존 바로가기가 존재해도 최신 경로로 덮어쓰기(Overwrite) 허용
5. **설치 완료 후 동작**:
   - 대안 A: 수동 실행 유도
   - 대안 B (채택안): Inno Setup 완료 페이지에서 `[x] GlazeBar + GlazeWM 시작` 체크박스를 기본 제공하여 즉시 재실행

## 4. 최종 확정된 결정 (Decision Outcome)
사용자와의 Grilling 인터뷰를 거쳐 5개 질문에 대해 만장일치로 합의되었습니다:
1. **구버전 사일런트 언인스톨**: 새 설치 전 기존 인스톨러(`UninstallString`) 및 `Uninstall.ps1`을 백그라운드 자동 실행하여 클린 설치 보장.
2. **프로세스 자동 종료**: 설치 시작 전 상주 중인 GlazeWM 프로세스군(`glazewm`, `glazewm-watcher`, `GlazeBar`, `GlazeTheme`, `WorkspaceToggle`)을 자동 종료하여 파일 잠김(PermissionDenied) 방지.
3. **파일 잠금 재시도 및 안전 제거 (Safe-RemoveFile)**: 백신 검사나 지연 해제로 인한 일시적 파일 잠김 시 프로세스 재종료 및 최대 5회 재시도 루프 적용, 실패 시에도 전체 설치 중단 대신 경고 처리.
4. **사용자 설정 보존**: 업데이트 시 기존 `config.yaml`과 `theme-state/` 설정을 임시 보존 후 설치 완료 시 복원.
5. **시작프로그램 바로가기 갱신**: 바로가기가 이미 존재하더라도 예외 없이 최신 경로로 덮어쓰도록 허용.
6. **설치 후 자동 재실행**: Inno Setup `[Run]` 항목을 추가하여 설치 직후 GlazeWM + GlazeBar를 즉시 시작할 수 있도록 구성.
