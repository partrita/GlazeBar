#Requires -Version 5.1
param(
    [Parameter(Mandatory)][ValidateSet('Install','Uninstall','CheckUninstall','StopProcesses')][string]$Action,
    [Parameter(Mandatory)][string]$Bundle,
    [switch]$AutoStart,
    [string]$TestProfile,
    [string]$Theme = ''
)
$ErrorActionPreference = 'Stop'
# A parent PowerShell 7 session can export incompatible module paths.
$env:PSModulePath = Join-Path $PSHOME 'Modules'
Import-Module Microsoft.PowerShell.Utility,Microsoft.PowerShell.Management,Microsoft.PowerShell.Security -ErrorAction Stop
$logRoot = Join-Path $env:LOCALAPPDATA 'GlazeBarSetup'
if ($TestProfile) { $logRoot = Join-Path $TestProfile '.setup-logs' }
New-Item -ItemType Directory -Force $logRoot | Out-Null
$logAction = if ($Action -eq 'CheckUninstall') { 'uninstall' } else { $Action.ToLower() }
$log = Join-Path $logRoot ($logAction + '.log')
function Stop-GlazeBarProcesses {
    $procNames = @('glazewm', 'glazewm-watcher', 'GlazeBar', 'GlazeTheme', 'WorkspaceToggle')
    $running = Get-Process -Name $procNames -ErrorAction SilentlyContinue
    if ($running) {
        cmd.exe /c 'taskkill /F /T /IM glazewm.exe /IM glazewm-watcher.exe /IM GlazeBar.exe /IM GlazeTheme.exe /IM WorkspaceToggle.exe >nul 2>&1'
        for ($attempt = 0; $attempt -lt 3; $attempt++) {
            $running = Get-Process -Name $procNames -ErrorAction SilentlyContinue
            if ($running) {
                $running | Stop-Process -Force -ErrorAction SilentlyContinue
                Start-Sleep -Milliseconds 400
            } else {
                break
            }
        }
    }
}

try {
    if ($Action -eq 'StopProcesses') {
        if (!$TestProfile) {
            Stop-GlazeBarProcesses
        }
        exit 0
    }
    if ($Action -eq 'CheckUninstall') {
        if (!$TestProfile) {
            Stop-GlazeBarProcesses
        }
        $profile = if ($TestProfile) { $TestProfile } else { $env:USERPROFILE }
        $statePath = Join-Path $profile '.glzr/glazewm/community-install.json'
        if (Test-Path -LiteralPath $statePath) {
            $state = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
            foreach ($entry in $state.entries) {
                if ($entry.existed -and !(Test-Path -LiteralPath (Join-Path $state.backup $entry.path))) {
                    Write-Warning "Backup missing: $($entry.path)"
                }
            }
        }
        exit 0
    }
    $arguments = @{}
    if ($TestProfile) { $arguments.StageOnly = $true; $arguments.ProfileRoot = $TestProfile }
    if ($Action -eq 'Install' -and $AutoStart -and !$TestProfile) { $arguments.AutoStart = $true }
    if ($Action -eq 'Install' -and $Theme) { $arguments.Theme = $Theme }
    & (Join-Path $Bundle ($Action + '.ps1')) @arguments *>&1 | Out-File -LiteralPath $log -Encoding UTF8
    exit 0
} catch {
    $_ | Out-String | Add-Content -LiteralPath $log -Encoding UTF8
    exit 1
}
