<#
.SYNOPSIS
Automated regression tests to prevent Antivirus (Windows Defender, AMSI) false positives
and CodeQL static analysis security alerts.
#>
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

function Assert-Condition {
    param(
        [bool]$Condition,
        [string]$Message
    )
    if (-not $Condition) {
        throw $Message
    }
}

function Read-RepoFile {
    param([string]$RelativePath)
    $full = Join-Path $RepoRoot $RelativePath
    return [System.IO.File]::ReadAllText($full, [System.Text.Encoding]::UTF8)
}

function Test-CargoReleaseProfile {
    Write-Host '[1/4] Checking Cargo release profile & linker flags...'
    $cargoToml = Read-RepoFile 'native-rust/Cargo.toml'

    Assert-Condition (-not $cargoToml.Contains('strip = true')) (
        "CRITICAL: Cargo.toml contains 'strip = true'. " +
        "Stripped Rust MSVC binaries trigger Windows Defender ML heuristics (Trojan:Win32/Wacatac.B!ml)."
    )
    Assert-Condition (-not $cargoToml.Contains('panic = "abort"')) (
        "CRITICAL: Cargo.toml contains 'panic = `"abort`"'. " +
        "Missing unwind tables alter PE structure and trigger AV heuristics."
    )
    Assert-Condition (-not $cargoToml.Contains('opt-level = "z"')) (
        "CRITICAL: Cargo.toml contains 'opt-level = `"z`"'. " +
        "Aggressive size optimization triggers packer/dropper detection."
    )
    Assert-Condition ($cargoToml.Contains('opt-level = 3')) "Cargo.toml should use safe 'opt-level = 3'."

    $cargoConfig = Read-RepoFile 'native-rust/.cargo/config.toml'
    Assert-Condition (-not $cargoConfig.Contains('/OPT:ICF')) (
        "CRITICAL: .cargo/config.toml contains '/OPT:ICF'. " +
        "Identical code folding changes PE section headers and triggers heuristic detection."
    )
    Assert-Condition (-not $cargoConfig.Contains('+crt-static')) (
        "CRITICAL: .cargo/config.toml contains '+crt-static'. " +
        "Static CRT combined with Win32 system APIs triggers Bitdefender Gen.Variant.Fugrafa false positives."
    )
    Assert-Condition ($cargoConfig.Contains('/RELEASE')) (
        "CRITICAL: .cargo/config.toml must specify '/RELEASE' linker flag to populate PE checksum."
    )

    $hooksRs = Read-RepoFile 'native-rust/src/hooks.rs'
    foreach ($bannedApi in @('SetWindowsHookExW', 'UnhookWindowsHookEx', 'CallNextHookEx', 'WH_KEYBOARD_LL', 'WH_MOUSE_LL', 'keybd_event', 'GetAsyncKeyState')) {
        Assert-Condition (-not $hooksRs.Contains($bannedApi)) (
            "CRITICAL: hooks.rs contains '$bannedApi'. " +
            "Low-level global input hooks and keystroke injection trigger Bitdefender Gen.Variant.Fugrafa keylogger heuristics."
        )
    }

    $nativeRs = Read-RepoFile 'native-rust/src/native.rs'
    Assert-Condition (-not $nativeRs.Contains('GetAsyncKeyState')) (
        "CRITICAL: native.rs must use GetKeyState instead of GetAsyncKeyState to avoid keylogger IAT heuristics."
    )

    $rcContent = Read-RepoFile 'native-rust/glazebar.rc'
    Assert-Condition ($rcContent.Contains('glazebar.manifest')) 'glazebar.rc must embed glazebar.manifest.'
    $manifestContent = Read-RepoFile 'native-rust/glazebar.manifest'
    Assert-Condition ($manifestContent.Contains('asInvoker')) 'glazebar.manifest must specify asInvoker.'

    Write-Host '  [OK] Cargo release profile and linker configuration are AV-safe.'
}

function Test-InstallerScripts {
    Write-Host '[2/4] Checking installer scripts for AMSI & AV heuristic triggers...'

    $installPs1 = Read-RepoFile 'distribution/Install.ps1'
    Assert-Condition (-not $installPs1.Contains('-ExecutionPolicy Bypass')) (
        "Install.ps1 must not use '-ExecutionPolicy Bypass' (known AV heuristic trigger)."
    )
    Assert-Condition ((-not $installPs1.Contains('Invoke-WebRequest')) -or (-not $installPs1.Contains('standalone-glazewm'))) (
        "CRITICAL: Install.ps1 must NOT download GlazeWM MSI dynamically via Invoke-WebRequest. " +
        "Downloading an installer to %TEMP% and running it via Start-Process triggers AMSI dropper heuristics."
    )
    Assert-Condition ($installPs1.Contains('$manifest.glazewmMsi')) (
        'Install.ps1 must use local bundled $manifest.glazewmMsi.'
    )

    $setupIss = Read-RepoFile 'distribution/GlazeBar-Setup.iss'
    Assert-Condition ($setupIss.Contains('-ExecutionPolicy RemoteSigned')) (
        "GlazeBar-Setup.iss must invoke PowerShell with '-ExecutionPolicy RemoteSigned'."
    )
    Assert-Condition (-not $setupIss.Contains('-ExecutionPolicy Bypass')) (
        "GlazeBar-Setup.iss must not contain '-ExecutionPolicy Bypass'."
    )
    Assert-Condition (-not $setupIss.Contains('LZMAUseSeparateProcess=yes')) (
        "GlazeBar-Setup.iss must not use 'LZMAUseSeparateProcess=yes'."
    )
    Assert-Condition (-not $setupIss.Contains('LZMANumBlockThreads=1')) (
        "GlazeBar-Setup.iss must not restrict LZMA threads to 1."
    )

    $bridgePs1 = Read-RepoFile 'distribution/Setup-Bridge.ps1'
    Assert-Condition (-not $bridgePs1.Contains('-ExecutionPolicy Bypass')) (
        "Setup-Bridge.ps1 must not use '-ExecutionPolicy Bypass'."
    )

    Write-Host '  [OK] Installer scripts contain no dropper or execution policy triggers.'
}

function Test-BundleRepackaging {
    Write-Host '[3/4] Checking bundle repackaging logic...'

    $repackagePs1 = Read-RepoFile 'distribution/Repackage-Bundle.ps1'
    Assert-Condition (-not $repackagePs1.Contains('Remove-Item -Recurse -Force $upstreamDir')) (
        "CRITICAL: Repackage-Bundle.ps1 must NOT delete upstreamDir. " +
        "The official signed GlazeWM MSI must remain bundled so the installer works offline without downloading."
    )
    Assert-Condition (-not $repackagePs1.Contains("glazewmMsi")) (
        "CRITICAL: Repackage-Bundle.ps1 must NOT remove glazewmMsi from manifest."
    )
    Assert-Condition ($repackagePs1.Contains('Normalize-Lockfile')) (
        "CRITICAL: Repackage-Bundle.ps1 must normalize Cargo.lock package version so version bumps do not abort slimming."
    )
    Assert-Condition ($repackagePs1.Contains('payload/GlazeTheme.exe')) (
        "CRITICAL: Repackage-Bundle.ps1 must remove the duplicate 5MB GlazeTheme.exe binary."
    )

    $workflowYml = Read-RepoFile '.github/workflows/package-installers.yml'
    Assert-Condition (-not $workflowYml.Contains('GlazeTheme.exe')) (
        'package-installers.yml must not package redundant duplicate GlazeTheme.exe.'
    )
    Assert-Condition ($workflowYml.Contains('throw "Repackage-Bundle.ps1 failed')) (
        'package-installers.yml must check $LASTEXITCODE and fail immediately if Repackage-Bundle.ps1 fails.'
    )
    Assert-Condition ($workflowYml.Contains('exceeds 20MB limit')) (
        'package-installers.yml must enforce bundle size limit (< 20MB) to prevent bloat.'
    )
    Assert-Condition ($workflowYml.Contains('exceeds 15MB limit')) (
        'package-installers.yml must enforce installer size limit (< 15MB).'
    )

    Write-Host '  [OK] Bundle repackaging retains upstream signed MSI and enforces slimming.'
}

function Test-CodeqlSafeguards {
    Write-Host '[4/4] Checking CodeQL static analysis patterns...'
    $wifiRs = Read-RepoFile 'native-rust/src/wifi.rs'

    Assert-Condition (-not $wifiRs.Contains('profile(&n, "")')) (
        "wifi.rs must not pass literal empty string '`"`"' to profile(...)."
    )
    Assert-Condition ($wifiRs.Contains('codeql[rust/hard-coded-cryptographic-value]')) (
        'wifi.rs should include CodeQL exclusion comment for password validation test.'
    )

    Write-Host '  [OK] CodeQL test patterns are properly constructed.'
}

Write-Host '=================================================================='
Write-Host '  Running GlazeBar Installer & AV Safety Regression Tests'
Write-Host '=================================================================='

try {
    Test-CargoReleaseProfile
    Test-InstallerScripts
    Test-BundleRepackaging
    Test-CodeqlSafeguards
    Write-Host '=================================================================='
    Write-Host '  [PASS] ALL SAFETY AND REGRESSION CHECKS PASSED.'
    Write-Host '=================================================================='
    exit 0
} catch {
    [Console]::Error.WriteLine("`n[FAIL] REGRESSION FAILURE:`n$($_.Exception.Message)`n")
    exit 1
}
