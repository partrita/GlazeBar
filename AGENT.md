# AGENT.md — GlazeBar 에이전트 개발 및 배포 가이드라인

이 문서는 AI 코딩 에이전트가 **GlazeBar** 저장소에서 코드를 수정, 빌드, 패키징, 릴리즈할 때 반드시 준수해야 하는 핵심 규칙을 정의합니다. 특히 **백신(Antivirus) 오탐 방지**와 **릴리즈 패키지 크기 최적화** 규칙은 어떠한 경우에도 위반해서는 안 됩니다.

---

## 1. 백신(Antivirus) 오탐 방지 필수 수칙 (최우선 순위)

서명되지 않은(Unsigned) Windows용 Rust 네이티브 바이너리와 PowerShell 기반 인스톨러는 Windows Defender(`Trojan:Win32/Wacatac.B!ml`), Bitdefender/알약(`Gen.Variant.Fugrafa`), AMSI 등의 휴리스틱/ML 엔진에 매우 민감합니다.

### 1.1 Rust 컴파일러 및 링커 설정 (`native-rust/Cargo.toml`, `native-rust/.cargo/config.toml`)
- **절대 사용 금지 설정**:
  - `strip = true` 금지: 심볼과 섹션 정보가 제거된 Rust MSVC 바이너리는 Windows Defender ML 엔진이 패킹된 악성코드(`Wacatac`)로 오탐합니다.
  - `panic = "abort"` 금지: 정상적인 Windows PE 언와인드 테이블(`.pdata`/`.xdata`)이 사라져 휴리스틱 점수가 급증합니다.
  - `opt-level = "z"` 또는 `"s"` 금지: 크기 압축 최적화는 코드 섹션 엔트로피를 비정상적으로 높입니다. 반드시 `opt-level = 3`을 유지하세요.
  - `/OPT:ICF` 링커 플래그 및 `UPX` 패커 사용 금지: PE 섹션 병합/압축은 즉각적인 오탐을 유발합니다.
  - `+crt-static` (정적 CRT 링크) 금지: 정적 CRT가 내장된 비서명 바이너리가 시스템 API를 호출하면 Bitdefender 엔진이 `Gen.Variant.Fugrafa`로 오탐합니다. 반드시 동적 CRT(`vcruntime140.dll`)를 사용하세요.
- **필수 유지 설정**:
  - `native-rust/.cargo/config.toml`에 `-C link-arg=/RELEASE`를 유지하여 PE 헤더 체크섬(Checksum)을 기록하세요.
  - `native-rust/glazebar.rc` 및 `native-rust/glazebar.manifest`를 통해 `requestedExecutionLevel level="asInvoker"`와 Windows 10/11 `supportedOS` 호환성 GUID 매니페스트를 항상 임베딩하세요.

### 1.2 Win32 API 임포트 테이블(IAT) 제한 규칙 (`native-rust/src/hooks.rs`, `native.rs`)
Bitdefender/알약 등 주요 백신은 실행 파일의 PE Import Address Table(IAT)에 키로거·인젝터 특징 API 조합이 존재하면 즉시 차단합니다.
- **절대 사용 금지 Win32 API**:
  - `SetWindowsHookExW`, `UnhookWindowsHookEx`, `CallNextHookEx`, `WH_KEYBOARD_LL`, `WH_MOUSE_LL` (전역 저수준 키보드/마우스 훅)
  - `keybd_event`, `SendInput` (가상 키 입력 주입)
  - `GetAsyncKeyState` (비동기 전역 하드웨어 키 상태 폴링)
- **표준 대체 방식**:
  - 전역 단축키(`Alt+M`, `Win+F1`, `Win+Alt+Shift+B`, `Win+Ctrl+Alt+Space`)는 반드시 표준 `RegisterHotKey` / `UnregisterHotKey` API와 `WM_HOTKEY` 메시지로 처리합니다.
  - 원격 데스크톱(`mstsc.exe`, `msrdc.exe`, `rdclientwpf.exe`) 활성화 감지, `Alt+Tab` 전환 완료(`EVENT_SYSTEM_SWITCHEND`), 외부 마우스 캡처(`EVENT_SYSTEM_CAPTURESTART`)는 `SetWinEventHook`으로 처리합니다.
  - 자체 창 내부(`WM_KEYDOWN`)에서의 보조키(`Ctrl`, `Shift`) 상태 확인은 메시지 큐 동기화 함수인 `GetKeyState`를 사용합니다.

### 1.3 인스톨러 및 PowerShell 스크립트 규칙 (`distribution/`)
- **실행 정책 우회 금지**: `Install.ps1`, `Uninstall.ps1`, `Setup-Bridge.ps1`, `GlazeBar-Setup.iss` 어디에서도 `-ExecutionPolicy Bypass`를 사용하지 마세요. Inno Setup에서는 `-ExecutionPolicy RemoteSigned`를 사용합니다.
- **런타임 MSI 다운로드(Dropper 패턴) 금지**: `Install.ps1`에서 `Invoke-WebRequest` 등으로 `standalone-glazewm-*.msi`를 `%TEMP%`에 내려받아 실행하면 AMSI가 `TrojanDownloader`로 차단합니다. GlazeWM 공식 서명 MSI(`upstream/standalone-glazewm-v3.10.1-*.msi`)는 반드시 번들 내부에 포함(`$manifest.glazewmMsi`)시켜 오프라인으로 실행해야 합니다.
- **Inno Setup 압축 옵션**: `GlazeBar-Setup.iss`에 `LZMAUseSeparateProcess=yes` 또는 `LZMANumBlockThreads=1`을 추가하지 마세요.

---

## 2. 릴리즈 패키지 크기 관리 및 번들 경량화 수칙

릴리즈 아티팩트가 비정상적으로 커지지 않도록(예: 130MB+ 폭증 사고 방지) 다음 패키징 규칙을 엄격히 유지합니다.

### 2.1 목표 패키지 크기 기준
- **Inno Setup 설치 파일 (`GlazeBar-GlazeWM-Setup-<ver>-<arch>.exe`)**: 약 **8 ~ 10 MB**
- **커뮤니티 번들 (`GlazeBar-Community-<ver>-windows-<arch>.zip`)**: 약 **10 ~ 14 MB**
- **단독 실행 파일 아카이브 (`GlazeBar-windows-<arch>.zip`)**: 약 **2 MB** (`GlazeBar.exe` 단일 포함)

### 2.2 중복 바이너리 및 오프라인 소스 아카이브 제거
1. **`GlazeTheme.exe` 중복 배포 금지**:
   - 과거 `GlazeBar.exe`와 100% 동일한 5MB 바이너리를 `GlazeTheme.exe`로 중복 복사하던 방식은 용량을 2배로 늘리고 백신 중복 탐지를 유발하므로 완전히 제거했습니다.
   - `payload/GlazeBar.exe` 단일 실행 파일만 배포하며, `Repackage-Bundle.py`는 베이스 번들에서 추출된 `payload/GlazeTheme.exe`를 반드시 삭제(`unlink(missing_ok=True)`)해야 합니다.
2. **`Repackage-Bundle.py --slim` 경량화 동작 보장**:
   - `--slim` 플래그 사용 시 약 135MB에 달하는 오프라인 빌드용 소스/벤더 아카이브(`source/*.zip`, `source/*.tar*`)와 약 6MB의 개별 크레이트 라이선스 선언 폴더(`licenses/glazebar/`, `licenses/glazewm/` 내 1,500여 개 파일)를 삭제합니다.
   - 단, 법적 고지 및 오프라인 설치에 필수적인 `LICENSE`, `THIRD-PARTY-NOTICES.txt`, `licenses/dependencies.json`, `licenses/standard/` 및 `upstream/standalone-glazewm-*.msi`는 **절대 삭제하지 마세요**.
3. **버전 변경 시 `Cargo.lock` 비교 호환성 유지**:
   - `Cargo.toml`의 패키지 버전을 올리면 `Cargo.lock`의 `glazebar-native` 버전도 함께 변경됩니다. `Repackage-Bundle.py`의 `normalize_lockfile()`을 유지하여 버전 문자열 차이로 리패키징이 중단되지 않도록 하세요.
4. **CI 워크플로우 에러 검증 필수 (`.github/workflows/package-installers.yml`)**:
   - `python distribution/Repackage-Bundle.py ... --slim` 호출 직후 반드시 `if ($LASTEXITCODE -ne 0) { throw ... }`로 종료 코드를 검증하세요. 리패키징 도중 예외가 발생했는데 CI가 무시하고 넘어가면, 슬리밍 전의 150MB 원본 번들이 그대로 인스톨러로 묶이게 됩니다.

---

## 3. 정적 보안 분석(CodeQL) 및 크로스 플랫폼 테스트 수칙

1. **CodeQL 하드코딩 암호 경고 방지 (`rust/hard-coded-cryptographic-value`)**:
   - `native-rust/src/wifi.rs` 등의 비밀번호/프로필 검증 함수 단위 테스트에서 문자열 리터럴(`""`, `"password"` 등)을 직접 인자로 전달하지 마세요. 런타임 생성 변수(`String::from_utf8(...)`)를 사용하고 필요한 경우 `// codeql[rust/hard-coded-cryptographic-value]` 주석을 유지하세요.
2. **Windows CI 콘솔 인코딩(`cp1252`) 호환성**:
   - GitHub Actions Windows 러너의 기본 콘솔 인코딩(`cp1252`)에서는 `✓`(`\u2713`), `✗` 같은 유니코드 기호 출력 시 `UnicodeEncodeError`가 발생합니다.
   - Python 스크립트(`distribution/*.py`)에서는 ASCII 태그(`[OK]`, `[PASS]`, `[FAIL]`)만 사용하고, 상단에 `sys.stdout.reconfigure(encoding="utf-8", errors="replace")`를 설정하세요.

---

## 4. 코드 수정 및 릴리즈 전 필수 검증 명령어

코드를 커밋하거나 태그를 푸시하기 전에 반드시 아래 명령어들을 실행하여 통과 여부를 확인하세요 (`pre-commit`, `pre-push` 훅에서도 자동 실행됩니다):

```bash
# 1. Rust 코드 포매팅 검사
cargo fmt --manifest-path native-rust/Cargo.toml --all --check

# 2. Windows 타겟 컴파일 및 단위/안전성 테스트 코드 검증
CARGO_TARGET_DIR=/tmp/target cargo check --tests --manifest-path native-rust/Cargo.toml --target x86_64-pc-windows-gnu

# 3. 백신 오탐 방지, 인스톨러 안전성, 번들 슬리밍 및 CodeQL 회귀 테스트
python3 distribution/Test-InstallerSafety.py
```

회귀 테스트는 다음 세 곳에 구현되어 있으며, 새로운 오탐 패턴이나 패키징 이슈 발견 시 세 곳 모두에 검증 항목을 추가해야 합니다:
- [`distribution/Test-InstallerSafety.py`](distribution/Test-InstallerSafety.py)
- [`native-rust/src/distribution_safety.rs`](native-rust/src/distribution_safety.rs)
- [`distribution/Test-Bundle.ps1`](distribution/Test-Bundle.ps1)
