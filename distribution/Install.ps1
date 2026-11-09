#Requires -Version 5.1
[CmdletBinding()]
param([switch]$AutoStart, [switch]$StageOnly, [string]$ProfileRoot = $env:USERPROFILE, [string]$Theme = '')
$ErrorActionPreference = 'Stop'
if (!$StageOnly -and [IO.Path]::GetFullPath($ProfileRoot) -ne [IO.Path]::GetFullPath($env:USERPROFILE)) { throw 'Custom ProfileRoot is for StageOnly testing.' }
if ($StageOnly -and [IO.Path]::GetFullPath($ProfileRoot) -eq [IO.Path]::GetFullPath($env:USERPROFILE)) { throw 'StageOnly requires an isolated ProfileRoot.' }
$manifest = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'manifest.json') -Raw | ConvertFrom-Json
foreach ($item in $manifest.files) {
    $file = Join-Path $PSScriptRoot $item.path
    if (!(Test-Path -LiteralPath $file) -or (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash -ne $item.sha256) { throw "Integrity check failed: $($item.path)" }
}
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

if (!$StageOnly) {
    $arch = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
    $expected = if ($manifest.architecture -eq 'arm64') { 'ARM64' } else { 'AMD64' }
    if ($arch -ne $expected) { throw "Use the package for your Windows architecture ($arch)." }
    Stop-GlazeBarProcesses
    if (![Environment]::Is64BitProcess) { throw 'Run this installer from 64-bit PowerShell.' }
    # The official GlazeWM MSI needs VC runtime even though GlazeBar links it statically.
    if (!(Test-Path -LiteralPath (Join-Path ([Environment]::SystemDirectory) 'vcruntime140.dll'))) {
        $download = Join-Path ([IO.Path]::GetTempPath()) ('glazebar-vcredist-' + [Guid]::NewGuid().ToString('N') + '.exe')
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -UseBasicParsing "https://aka.ms/vc14/vc_redist.$($manifest.architecture).exe" -OutFile $download
        $signature = Get-AuthenticodeSignature -LiteralPath $download
        if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation') { throw 'Microsoft runtime signature verification failed.' }
        Write-Output 'GlazeWM requires the Microsoft VC runtime. Complete the Microsoft installer and its license dialog.'
        $runtime = Start-Process -FilePath $download -ArgumentList @('/install','/norestart') -Wait -PassThru
        if ($runtime.ExitCode -notin @(0,3010,1638)) { throw "Microsoft runtime installation failed or was cancelled: $($runtime.ExitCode)" }
    }
}
$target = Join-Path $ProfileRoot '.glzr\glazewm'
$stateFile = Join-Path $target 'community-install.json'
$userConfigPath = Join-Path $target 'config.yaml'
$userThemeStatePath = Join-Path $target 'theme-state'
$savedUserConfig = $null
$savedThemeStateDir = $null

if (Test-Path -LiteralPath $stateFile) {
    Write-Output 'Existing community installation detected. Preserving settings and uninstalling previous version...'
    if (Test-Path -LiteralPath $userConfigPath) {
        $savedUserConfig = [IO.File]::ReadAllText($userConfigPath)
    }
    if (Test-Path -LiteralPath $userThemeStatePath) {
        $savedThemeStateDir = Join-Path ([IO.Path]::GetTempPath()) ('glazebar-theme-state-' + [Guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Force $savedThemeStateDir | Out-Null
        Copy-Item -LiteralPath $userThemeStatePath -Destination (Join-Path $savedThemeStateDir 'theme-state') -Recurse -Force
    }
    $uninstallScript = Join-Path $PSScriptRoot 'Uninstall.ps1'
    $uninstallArgs = @{}
    if ($StageOnly) { $uninstallArgs.StageOnly = $true }
    $uninstallArgs.ProfileRoot = $ProfileRoot
    & $uninstallScript @uninstallArgs
}
function Safe-CopyFile([string]$src, [string]$dst) {
    New-Item -ItemType Directory -Force (Split-Path $dst) | Out-Null
    try {
        Copy-Item -LiteralPath $src -Destination $dst -Force -ErrorAction Stop
        return
    } catch {}

    if (!$StageOnly) {
        Stop-GlazeBarProcesses
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
function Safe-RemoveFile([string]$path) {
    if (Test-Path -LiteralPath $path) {
        try {
            Remove-Item -LiteralPath $path -Force -ErrorAction Stop
            return
        } catch {}

        if (!$StageOnly) {
            Stop-GlazeBarProcesses
        }

        try {
            Remove-Item -LiteralPath $path -Force -ErrorAction Stop
            return
        } catch {}

        try {
            $oldPath = $path + '.' + [Guid]::NewGuid().ToString('N') + '.old'
            [IO.File]::Move($path, $oldPath)
            Remove-Item -LiteralPath $oldPath -Force -ErrorAction SilentlyContinue
        } catch {
            Write-Warning "Could not remove or rename: $path ($($_.Exception.Message))"
        }
    }
}
$backup = Join-Path $ProfileRoot ('.glzr\community-backups\' + [Guid]::NewGuid().ToString('N'))
$payload = Join-Path $PSScriptRoot 'payload'
$entries = @()
$shortcutPath = $null
$shortcutCreated = $false
try {
    foreach ($file in Get-ChildItem -LiteralPath $payload -Recurse -File) {
        $relative = $file.FullName.Substring($payload.Length + 1)
        $dest = Join-Path $target $relative
        $saved = Join-Path $backup $relative
        $existed = Test-Path -LiteralPath $dest
        if ($existed) { Safe-CopyFile $dest $saved }
        $entry = [ordered]@{ path = $relative; existed = $existed; sha256 = '' }
        $entries += $entry
        if ($relative -eq 'config.yaml') {
            New-Item -ItemType Directory -Force (Split-Path $dest) | Out-Null
            $exe = (Join-Path $target 'GlazeBar.exe').Replace("'", "''")
            $text = [IO.File]::ReadAllText($file.FullName).Replace('__GLAZEBAR_EXE__', $exe)
            [IO.File]::WriteAllText($dest, $text, (New-Object Text.UTF8Encoding $false))
        } else {
            Safe-CopyFile $file.FullName $dest
        }
        $entry.sha256 = (Get-FileHash -LiteralPath $dest).Hash
    }
    if ($savedUserConfig) {
        $destConfig = Join-Path $target 'config.yaml'
        $exe = (Join-Path $target 'GlazeBar.exe').Replace("'", "''")
        $restoredText = $savedUserConfig.Replace('__GLAZEBAR_EXE__', $exe)
        [IO.File]::WriteAllText($destConfig, $restoredText, (New-Object Text.UTF8Encoding $false))
    }
    if ($savedThemeStateDir -and (Test-Path -LiteralPath (Join-Path $savedThemeStateDir 'theme-state'))) {
        Copy-Item -LiteralPath (Join-Path $savedThemeStateDir 'theme-state') -Destination $target -Recurse -Force
        Remove-Item -LiteralPath $savedThemeStateDir -Recurse -Force -ErrorAction SilentlyContinue
    } elseif ($Theme) {
        $schemes = @{
            'tokyo-night' = @{
                mode = 'tokyo-night'; name = 'GlazeBar Aether'
                palette = @('#1a1b26','#f7768e','#9ece6a','#e0af68','#7aa2f7','#bb9af7','#7dcfff','#a9b1d6','#8992b8','#ff899d','#b9f27c','#ff9e64','#8db0ff','#c0caf5','#b4f9f8','#c0caf5')
            }
            'catppuccin' = @{
                mode = 'catppuccin'; name = 'GlazeBar Aether'
                palette = @('#1e1e2e','#f38ba8','#a6e3a1','#f9e2af','#89b4fa','#f5c2e7','#94e2d5','#bac2de','#9399b2','#f7aec2','#c2ecbf','#fcd682','#aeccfc','#cba6f7','#b1eae1','#cdd6f4')
            }
            'everforest' = @{
                mode = 'everforest'; name = 'GlazeBar Aether'
                palette = @('#2d353b','#e67e80','#a7c080','#dbbc7f','#7fbbb3','#d699b6','#83c092','#d3c6aa','#9da9a0','#e67e80','#a7c080','#dbbc7f','#7fbbb3','#d699b6','#83c092','#d3c6aa')
            }
            'rose-pine' = @{
                mode = 'rose-pine'; name = 'GlazeBar Aether'
                palette = @('#191724','#eb6f92','#31748f','#f6c177','#9ccfd8','#c4a7e7','#ebbcba','#e0def4','#908caa','#eb6f92','#31748f','#f6c177','#9ccfd8','#c4a7e7','#ebbcba','#e0def4')
            }
            'monokai' = @{
                mode = 'monokai'; name = 'GlazeBar Aether'
                palette = @('#272822','#f92672','#a6e22e','#f4bf75','#66d9ef','#ae81ff','#a1efe4','#f8f8f2','#9a9b94','#fd5ff0','#b6e354','#feed6c','#8cedff','#9e6ffe','#899ca1','#f9f8f5')
            }
            'dracula' = @{
                mode = 'dracula'; name = 'GlazeBar Aether'
                palette = @('#282a36','#ff5555','#50fa7b','#f1fa8c','#bd93f9','#ff79c6','#8be9fd','#f8f8f2','#92a2d4','#ff6e6e','#69ff94','#ffffa5','#d6acff','#ff92df','#a4ffff','#ffffff')
            }
        }
        $selected = $schemes[$Theme.ToLower()]
        if ($selected) {
            $stateDir = Join-Path $target 'theme-state'
            New-Item -ItemType Directory -Force $stateDir | Out-Null
            $palFile = Join-Path $stateDir 'current-palette.json'
            if (!(Test-Path -LiteralPath $palFile)) {
                $payload = [ordered]@{
                    image = ''
                    light = $false
                    mode = $selected.mode
                    name = $selected.name
                    palette = $selected.palette
                }
                $payload | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $palFile -Encoding UTF8
            }
        }
    }
    if (!$StageOnly) {
        $cli = Join-Path $env:ProgramFiles 'glzr.io\GlazeWM\cli\glazewm.exe'
        if (Test-Path -LiteralPath $cli) {
            $version = & $cli --version
            if ($version -and $version.Trim() -ne 'glazewm 3.10.1') {
                Write-Warning "Existing GlazeWM version ($($version.Trim())) detected. GlazeBar is tested with 3.10.1."
            }
        } else {
            $msi = Join-Path $PSScriptRoot $manifest.glazewmMsi
            $process = Start-Process msiexec.exe -ArgumentList @('/i', ('"' + $msi + '"'), '/passive', '/norestart') -Wait -PassThru -WindowStyle Hidden
            if ($process.ExitCode -notin @(0,3010)) { throw "GlazeWM installation failed: $($process.ExitCode)" }
            if (!(Test-Path -LiteralPath $cli)) { throw 'GlazeWM CLI was not found after installation.' }
        }
        if ($AutoStart) {
            $shortcutPath = Join-Path ([Environment]::GetFolderPath('Startup')) 'GlazeBar Community.lnk'
            $link = (New-Object -ComObject WScript.Shell).CreateShortcut($shortcutPath)
            $link.TargetPath = Join-Path $target 'wm\glazewm.exe'
            $link.Arguments = 'start'
            $link.WorkingDirectory = Join-Path $target 'wm'
            $link.Save()
            $shortcutCreated = $true
        }
    }
    [ordered]@{ version = $manifest.version; backup = $backup; entries = $entries; startupShortcut = $shortcutPath; staged = [bool]$StageOnly } |
        ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $stateFile -Encoding UTF8
    Write-Output "Installed to $target. Start GlazeWM from the Start menu. Automatic startup: $([bool]$AutoStart)."
} catch {
    foreach ($entry in $entries) {
        $dest = Join-Path $target $entry.path
        if ($entry.existed) { Safe-CopyFile (Join-Path $backup $entry.path) $dest }
        elseif (Test-Path -LiteralPath $dest) { Safe-RemoveFile $dest }
    }
    if ($shortcutCreated -and (Test-Path -LiteralPath $shortcutPath)) { Safe-RemoveFile $shortcutPath }
    throw
}
