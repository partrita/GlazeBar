#!/usr/bin/env python3
"""
Test-InstallerSafety.py
Automated regression tests to prevent Antivirus (Windows Defender, AMSI) false positives
and CodeQL static analysis security alerts.
"""

import sys
import re
from pathlib import Path

# Ensure stdout/stderr handle any encoding gracefully
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

REPO_ROOT = Path(__file__).resolve().parent.parent

def test_cargo_release_profile():
    print("[1/4] Checking Cargo release profile & linker flags...")
    cargo_toml = (REPO_ROOT / "native-rust/Cargo.toml").read_text(encoding="utf-8")
    
    assert "strip = true" not in cargo_toml, (
        "CRITICAL: Cargo.toml contains 'strip = true'. "
        "Stripped Rust MSVC binaries trigger Windows Defender ML heuristics (Trojan:Win32/Wacatac.B!ml)."
    )
    assert "panic = \"abort\"" not in cargo_toml, (
        "CRITICAL: Cargo.toml contains 'panic = \"abort\"'. "
        "Missing unwind tables alter PE structure and trigger AV heuristics."
    )
    assert "opt-level = \"z\"" not in cargo_toml, (
        "CRITICAL: Cargo.toml contains 'opt-level = \"z\"'. "
        "Aggressive size optimization triggers packer/dropper detection."
    )
    assert 'opt-level = 3' in cargo_toml, "Cargo.toml should use safe 'opt-level = 3'."
    
    cargo_config = (REPO_ROOT / "native-rust/.cargo/config.toml").read_text(encoding="utf-8")
    assert "/OPT:ICF" not in cargo_config, (
        "CRITICAL: .cargo/config.toml contains '/OPT:ICF'. "
        "Identical code folding changes PE section headers and triggers heuristic detection."
    )
    assert "+crt-static" not in cargo_config, (
        "CRITICAL: .cargo/config.toml contains '+crt-static'. "
        "Static CRT combined with Win32 system APIs triggers Bitdefender Gen.Variant.Fugrafa false positives."
    )
    assert "/RELEASE" in cargo_config, (
        "CRITICAL: .cargo/config.toml must specify '/RELEASE' linker flag to populate PE checksum."
    )
    
    hooks_rs = (REPO_ROOT / "native-rust/src/hooks.rs").read_text(encoding="utf-8")
    for banned_api in ["SetWindowsHookExW", "UnhookWindowsHookEx", "CallNextHookEx", "WH_KEYBOARD_LL", "WH_MOUSE_LL", "keybd_event", "GetAsyncKeyState"]:
        assert banned_api not in hooks_rs, (
            f"CRITICAL: hooks.rs contains '{banned_api}'. "
            "Low-level global input hooks and keystroke injection trigger Bitdefender Gen.Variant.Fugrafa keylogger heuristics."
        )
    native_rs = (REPO_ROOT / "native-rust/src/native.rs").read_text(encoding="utf-8")
    assert "GetAsyncKeyState" not in native_rs, (
        "CRITICAL: native.rs must use GetKeyState instead of GetAsyncKeyState to avoid keylogger IAT heuristics."
    )
    
    rc_content = (REPO_ROOT / "native-rust/glazebar.rc").read_text(encoding="utf-8")
    assert "glazebar.manifest" in rc_content, "glazebar.rc must embed glazebar.manifest."
    manifest_content = (REPO_ROOT / "native-rust/glazebar.manifest").read_text(encoding="utf-8")
    assert "asInvoker" in manifest_content, "glazebar.manifest must specify asInvoker."
    print("  [OK] Cargo release profile and linker configuration are AV-safe.")

def test_installer_scripts():
    print("[2/4] Checking installer scripts for AMSI & AV heuristic triggers...")
    
    # 1. Install.ps1
    install_ps1 = (REPO_ROOT / "distribution/Install.ps1").read_text(encoding="utf-8")
    assert "-ExecutionPolicy Bypass" not in install_ps1, (
        "Install.ps1 must not use '-ExecutionPolicy Bypass' (known AV heuristic trigger)."
    )
    assert "Invoke-WebRequest" not in install_ps1 or "standalone-glazewm" not in install_ps1, (
        "CRITICAL: Install.ps1 must NOT download GlazeWM MSI dynamically via Invoke-WebRequest. "
        "Downloading an installer to %TEMP% and running it via Start-Process triggers AMSI dropper heuristics."
    )
    assert "$manifest.glazewmMsi" in install_ps1, (
        "Install.ps1 must use local bundled $manifest.glazewmMsi."
    )
    
    # 2. GlazeBar-Setup.iss
    setup_iss = (REPO_ROOT / "distribution/GlazeBar-Setup.iss").read_text(encoding="utf-8")
    assert "-ExecutionPolicy RemoteSigned" in setup_iss, (
        "GlazeBar-Setup.iss must invoke PowerShell with '-ExecutionPolicy RemoteSigned'."
    )
    assert "-ExecutionPolicy Bypass" not in setup_iss, (
        "GlazeBar-Setup.iss must not contain '-ExecutionPolicy Bypass'."
    )
    assert "LZMAUseSeparateProcess=yes" not in setup_iss, (
        "GlazeBar-Setup.iss must not use 'LZMAUseSeparateProcess=yes'."
    )
    assert "LZMANumBlockThreads=1" not in setup_iss, (
        "GlazeBar-Setup.iss must not restrict LZMA threads to 1."
    )
    
    # 3. Setup-Bridge.ps1
    bridge_ps1 = (REPO_ROOT / "distribution/Setup-Bridge.ps1").read_text(encoding="utf-8")
    assert "-ExecutionPolicy Bypass" not in bridge_ps1, (
        "Setup-Bridge.ps1 must not use '-ExecutionPolicy Bypass'."
    )
    print("  [OK] Installer scripts contain no dropper or execution policy triggers.")

def test_bundle_repackaging():
    print("[3/4] Checking bundle repackaging logic...")
    repackage_py = (REPO_ROOT / "distribution/Repackage-Bundle.py").read_text(encoding="utf-8")
    assert "shutil.rmtree(upstream_dir" not in repackage_py, (
        "CRITICAL: Repackage-Bundle.py must NOT delete upstream_dir. "
        "The official signed GlazeWM MSI must remain bundled so the installer works offline without downloading."
    )
    assert "manifest.pop('glazewmMsi'" not in repackage_py, (
        "CRITICAL: Repackage-Bundle.py must NOT remove glazewmMsi from manifest."
    )
    assert "normalize_lockfile" in repackage_py, (
        "CRITICAL: Repackage-Bundle.py must normalize Cargo.lock package version so version bumps do not abort slimming."
    )
    assert "(folder / 'payload/GlazeTheme.exe').unlink(missing_ok=True)" in repackage_py, (
        "CRITICAL: Repackage-Bundle.py must remove the duplicate 5MB GlazeTheme.exe binary."
    )
    workflow_yml = (REPO_ROOT / ".github/workflows/package-installers.yml").read_text(encoding="utf-8")
    assert "GlazeTheme.exe" not in workflow_yml, (
        "package-installers.yml must not package redundant duplicate GlazeTheme.exe."
    )
    assert 'throw "Repackage-Bundle.py failed' in workflow_yml, (
        "package-installers.yml must check $LASTEXITCODE and fail immediately if Repackage-Bundle.py fails."
    )
    assert "exceeds 20MB limit" in workflow_yml, (
        "package-installers.yml must enforce bundle size limit (< 20MB) to prevent bloat."
    )
    assert "exceeds 15MB limit" in workflow_yml, (
        "package-installers.yml must enforce installer size limit (< 15MB)."
    )
    print("  [OK] Bundle repackaging retains upstream signed MSI and enforces slimming.")

def test_codeql_safeguards():
    print("[4/4] Checking CodeQL static analysis patterns...")
    wifi_rs = (REPO_ROOT / "native-rust/src/wifi.rs").read_text(encoding="utf-8")
    
    assert 'profile(&n, "")' not in wifi_rs, (
        "wifi.rs must not pass literal empty string '\"\"' to profile(...)."
    )
    assert 'codeql[rust/hard-coded-cryptographic-value]' in wifi_rs, (
        "wifi.rs should include CodeQL exclusion comment for password validation test."
    )
    print("  [OK] CodeQL test patterns are properly constructed.")

def main():
    print("==================================================================")
    print("  Running GlazeBar Installer & AV Safety Regression Tests")
    print("==================================================================")
    try:
        test_cargo_release_profile()
        test_installer_scripts()
        test_bundle_repackaging()
        test_codeql_safeguards()
        print("==================================================================")
        print("  [PASS] ALL SAFETY AND REGRESSION CHECKS PASSED.")
        print("==================================================================")
        return 0
    except AssertionError as e:
        print(f"\n[FAIL] REGRESSION FAILURE:\n{e}\n", file=sys.stderr)
        return 1

if __name__ == "__main__":
    sys.exit(main())
