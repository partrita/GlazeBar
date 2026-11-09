#Requires -Version 5.1
[CmdletBinding()]
param([switch]$StageOnly, [string]$ProfileRoot = $env:USERPROFILE)
$ErrorActionPreference = 'Stop'
if (!$StageOnly -and [IO.Path]::GetFullPath($ProfileRoot) -ne [IO.Path]::GetFullPath($env:USERPROFILE)) { throw 'Custom ProfileRoot is for StageOnly testing.' }
if ($StageOnly -and [IO.Path]::GetFullPath($ProfileRoot) -eq [IO.Path]::GetFullPath($env:USERPROFILE)) { throw 'StageOnly requires an isolated ProfileRoot.' }
if (!$StageOnly) {
    & taskkill.exe /F /T /IM glazewm.exe /IM glazewm-watcher.exe /IM GlazeBar.exe /IM GlazeTheme.exe /IM WorkspaceToggle.exe 2>$null | Out-Null
    for ($attempt = 0; $attempt -lt 3; $attempt++) {
        $running = Get-Process -Name glazewm,glazewm-watcher,GlazeBar,GlazeTheme,WorkspaceToggle -ErrorAction SilentlyContinue
        if ($running) {
            $running | Stop-Process -Force -ErrorAction SilentlyContinue
            Start-Sleep -Milliseconds 400
        } else {
            break
        }
    }
}
function Safe-RemoveFile([string]$path) {
    if (Test-Path -LiteralPath $path) {
        try {
            Remove-Item -LiteralPath $path -Force -ErrorAction Stop
            return
        } catch {}

        if (!$StageOnly) {
            & taskkill.exe /F /T /IM glazewm.exe /IM glazewm-watcher.exe /IM GlazeBar.exe /IM GlazeTheme.exe /IM WorkspaceToggle.exe 2>$null | Out-Null
            Get-Process -Name glazewm,glazewm-watcher,GlazeBar,GlazeTheme,WorkspaceToggle -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
            Start-Sleep -Milliseconds 400
        }

        try {
            Remove-Item -LiteralPath $path -Force -ErrorAction Stop
            return
        } catch {}

        # In Windows NTFS, in-use executables can be moved/renamed within the same volume even while open
        try {
            $oldPath = $path + '.' + [Guid]::NewGuid().ToString('N') + '.old'
            [IO.File]::Move($path, $oldPath)
            Remove-Item -LiteralPath $oldPath -Force -ErrorAction SilentlyContinue
        } catch {
            Write-Warning "Could not remove or rename: $path ($($_.Exception.Message))"
        }
    }
}
function Safe-CopyFile([string]$src, [string]$dst) {
    New-Item -ItemType Directory -Force (Split-Path $dst) | Out-Null
    try {
        Copy-Item -LiteralPath $src -Destination $dst -Force -ErrorAction Stop
        return
    } catch {}

    if (!$StageOnly) {
        & taskkill.exe /F /T /IM glazewm.exe /IM glazewm-watcher.exe /IM GlazeBar.exe /IM GlazeTheme.exe /IM WorkspaceToggle.exe 2>$null | Out-Null
        Get-Process -Name glazewm,glazewm-watcher,GlazeBar,GlazeTheme,WorkspaceToggle -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 400
    }

    try {
        Copy-Item -LiteralPath $src -Destination $dst -Force -ErrorAction Stop
        return
    } catch {}

    try {
        $oldDst = $dst + '.' + [Guid]::NewGuid().ToString('N') + '.old'
        [IO.File]::Move($dst, $oldDst)
        Remove-Item -LiteralPath $oldDst -Force -ErrorAction SilentlyContinue
    } catch {}

    Copy-Item -LiteralPath $src -Destination $dst -Force
}
$target = [IO.Path]::GetFullPath((Join-Path $ProfileRoot '.glzr\glazewm'))
$stateFile = Join-Path $target 'community-install.json'
$state = Get-Content -LiteralPath $stateFile -Raw | ConvertFrom-Json
if ([bool]$state.staged -ne [bool]$StageOnly) { throw 'Installation mode does not match.' }
$backupRoot = [IO.Path]::GetFullPath((Join-Path $ProfileRoot '.glzr\community-backups')) + '\'
$backup = [IO.Path]::GetFullPath($state.backup)
if (!$backup.StartsWith($backupRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Invalid backup location.' }
$recovery = Join-Path $backup 'edits-at-uninstall'
# Validate every path and backup before changing anything.
foreach ($entry in $state.entries) {
    $dest = [IO.Path]::GetFullPath((Join-Path $target $entry.path))
    if (!$dest.StartsWith($target + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Invalid manifest path.' }
    if ($entry.existed -and !(Test-Path -LiteralPath (Join-Path $backup $entry.path))) {
        Write-Warning "Missing backup: $($entry.path)"
    }
}
foreach ($entry in $state.entries) {
    $dest = Join-Path $target $entry.path
    if ((Test-Path -LiteralPath $dest) -and (Get-FileHash -LiteralPath $dest).Hash -ne $entry.sha256) {
        $saved = Join-Path $recovery $entry.path
        Safe-CopyFile $dest $saved
    }
    if ($entry.existed) {
        $backupFile = Join-Path $backup $entry.path
        if (Test-Path -LiteralPath $backupFile) {
            Safe-CopyFile $backupFile $dest
        }
    }
    elseif (Test-Path -LiteralPath $dest) { Safe-RemoveFile $dest }
}
if (!$StageOnly -and $state.startupShortcut) {
    $allowed = Join-Path ([Environment]::GetFolderPath('Startup')) 'GlazeBar Community.lnk'
    if ($state.startupShortcut -eq $allowed -and (Test-Path -LiteralPath $allowed)) { Safe-RemoveFile $allowed }
}
Safe-RemoveFile $stateFile
Write-Output "Previous files restored. Backups and later edits retained at $backup. GlazeWM remains installed; remove it through Windows Installed apps if desired."
