<#
.SYNOPSIS
Update a verified architecture bundle with GlazeBar and, optionally, a rebuilt modified GlazeWM.
Dependency vendors, licenses and the official GlazeWM MSI are always retained.
Without -WmRuntime the modified GlazeWM from the base bundle is kept as well.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Base,

    [Parameter(Mandatory = $true)]
    [string]$Bar,

    [Parameter(Mandatory = $true)]
    [string]$Output,

    [string]$Version = '0.0.11',

    [switch]$Slim,

    [string]$WmRuntime,

    [string]$WmSource,

    [string]$WmVersion
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Get-BytesSha256 {
    param([byte[]]$Bytes)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $hash = $sha.ComputeHash($Bytes)
        return ([System.BitConverter]::ToString($hash) -replace '-', '').ToUpperInvariant()
    } finally {
        $sha.Dispose()
    }
}

function Get-FileSha256 {
    param([string]$Path)
    return (Get-FileHash -Path $Path -Algorithm SHA256).Hash.ToUpperInvariant()
}

function Get-PeMachine {
    param([byte[]]$Bytes)
    if ($Bytes.Length -lt 64) {
        throw 'Binary is too small to be a valid PE file.'
    }
    $peOffset = [System.BitConverter]::ToInt32($Bytes, 60)
    if ($peOffset -lt 0 -or ($peOffset + 6) -gt $Bytes.Length) {
        throw 'Invalid PE header offset.'
    }
    return [System.BitConverter]::ToUInt16($Bytes, $peOffset + 4)
}

function Read-ZipEntryBytes {
    param(
        [System.IO.Compression.ZipArchive]$Archive,
        [string]$EntryName
    )
    $entry = $Archive.GetEntry($EntryName)
    if (-not $entry) {
        throw "Zip entry not found: $EntryName"
    }
    $stream = $entry.Open()
    $ms = New-Object System.IO.MemoryStream
    try {
        $stream.CopyTo($ms)
        return $ms.ToArray()
    } finally {
        $ms.Dispose()
        $stream.Dispose()
    }
}

function Write-ZipEntryBytes {
    param(
        [System.IO.Compression.ZipArchive]$Archive,
        [string]$EntryName,
        [byte[]]$Bytes
    )
    $entry = $Archive.CreateEntry($EntryName, [System.IO.Compression.CompressionLevel]::Optimal)
    $stream = $entry.Open()
    try {
        $stream.Write($Bytes, 0, $Bytes.Length)
    } finally {
        $stream.Dispose()
    }
}

function Normalize-Lockfile {
    param([byte[]]$Bytes)
    $text = [System.Text.Encoding]::UTF8.GetString($Bytes)
    $text = $text -replace "`r`n", "`n"
    $text = [regex]::Replace(
        $text,
        '(name = "glazebar-native"\nversion = )"[^"]+"',
        '$1"0.0.0"'
    )
    return $text
}

function Write-Utf8NoBom {
    param(
        [string]$Path,
        [string]$Content
    )
    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Content, $utf8NoBom)
}

function Get-GitTrackedFiles {
    param([string]$WorkingDir)
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = 'git'
    $psi.Arguments = 'ls-files -z'
    $psi.WorkingDirectory = $WorkingDir
    $psi.RedirectStandardOutput = $true
    $psi.UseShellExecute = $false
    $proc = [System.Diagnostics.Process]::Start($psi)
    $ms = New-Object System.IO.MemoryStream
    $proc.StandardOutput.BaseStream.CopyTo($ms)
    $proc.WaitForExit()
    if ($proc.ExitCode -ne 0) {
        throw "git ls-files failed with exit code $($proc.ExitCode)"
    }
    $raw = [System.Text.Encoding]::UTF8.GetString($ms.ToArray())
    $ms.Dispose()
    return @($raw.Split([char]0, [System.StringSplitOptions]::RemoveEmptyEntries))
}

function Invoke-ReplaceModifiedWm {
    param(
        [string]$Folder,
        [string]$Repo,
        [string]$Arch
    )
    $machineMap = @{ 'arm64' = 0xAA64; 'x64' = 0x8664 }
    $targetMap = @{ 'arm64' = 'aarch64-pc-windows-msvc'; 'x64' = 'x86_64-pc-windows-msvc' }
    $machine = $machineMap[$Arch]
    $target = $targetMap[$Arch]

    foreach ($name in @('glazewm.exe', 'glazewm-watcher.exe')) {
        $binPath = Join-Path $WmRuntime $name
        $binBytes = [System.IO.File]::ReadAllBytes($binPath)
        if ((Get-PeMachine -Bytes $binBytes) -ne $machine) {
            throw "Wrong GlazeWM binary architecture: $name"
        }
    }

    $toolchain = (& rustc +nightly --version).Trim()
    $tracked = Get-GitTrackedFiles -WorkingDir $WmSource
    $names = @($tracked + '.cargo/config.toml' | Sort-Object -Unique)
    $sourceZip = Join-Path $Folder 'source/glazewm-modified-source.zip'

    $oldArchive = [System.IO.Compression.ZipFile]::OpenRead($sourceZip)
    try {
        $oldLock = ([System.Text.Encoding]::UTF8.GetString((Read-ZipEntryBytes -Archive $oldArchive -EntryName 'Cargo.lock'))) -replace "`r`n", "`n"
    } finally {
        $oldArchive.Dispose()
    }
    $newLock = ([System.IO.File]::ReadAllText((Join-Path $WmSource 'Cargo.lock'), [System.Text.Encoding]::UTF8)) -replace "`r`n", "`n"
    if ($oldLock -ne $newLock) {
        throw 'Cannot reuse GlazeWM vendors with a different lockfile'
    }

    $tempZip = "$sourceZip.new"
    if (Test-Path $tempZip) { Remove-Item -Force $tempZip }
    $newArchive = [System.IO.Compression.ZipFile]::Open($tempZip, [System.IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($name in $names) {
            $full = Join-Path $WmSource $name
            if (Test-Path -LiteralPath $full -PathType Leaf) {
                [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($newArchive, $full, $name, [System.IO.Compression.CompressionLevel]::Optimal) | Out-Null
            }
        }
    } finally {
        $newArchive.Dispose()
    }
    Move-Item -Force $tempZip $sourceZip

    foreach ($name in @('glazewm.exe', 'glazewm-watcher.exe')) {
        Copy-Item -Force (Join-Path $WmRuntime $name) (Join-Path $Folder "payload/wm/$name")
    }
    $patch = Join-Path $Repo 'glazewm-patches/glazewm-v3.10.1.patch'
    Copy-Item -Force $patch (Join-Path $Folder 'source/glazewm-v3.10.1.patch')
    Copy-Item -Force (Join-Path $Repo 'docs/glazewm-modifications.md') (Join-Path $Folder 'source/GLAZEWM-MODIFICATIONS.md')

    foreach ($stale in @('BUILD-PREVIEW4.json', 'BUILD-PREVIEW5.json')) {
        $stalePath = Join-Path $Folder "source/$stale"
        if (Test-Path $stalePath) { Remove-Item -Force $stalePath }
    }

    $buildNotes = @(
        'Extract glazewm-modified-source.zip into a build folder.'
        'Extract glazewm-vendor.zip INTO its vendor/ subfolder (beside Cargo.toml).'
        'Run in an MSVC developer shell for the target architecture (Windows SDK required):'
        "set VERSION_NUMBER=$WmVersion"
        "cargo +nightly build --offline --locked --release --target $target -p wm --bin glazewm -p wm-watcher --bin glazewm-watcher"
        "Built with: $toolchain"
        ''
    ) -join "`n"
    Write-Utf8NoBom -Path (Join-Path $Folder 'source/BUILD-MODIFIED-WM.txt') -Content $buildNotes

    return @{
        glazewmUpstream        = 'https://github.com/glzr-io/glazewm/tree/v3.10.1'
        modifiedWmSourceSHA256 = Get-FileSha256 -Path $sourceZip
        wmPatchSHA256          = Get-FileSha256 -Path $patch
        wmSHA256               = Get-FileSha256 -Path (Join-Path $WmRuntime 'glazewm.exe')
        wmWatcherSHA256        = Get-FileSha256 -Path (Join-Path $WmRuntime 'glazewm-watcher.exe')
        wmToolchain            = $toolchain
    }
}

$hasWmRuntime = -not [string]::IsNullOrWhiteSpace($WmRuntime)
$hasWmSource = -not [string]::IsNullOrWhiteSpace($WmSource)
$hasWmVersion = -not [string]::IsNullOrWhiteSpace($WmVersion)
if (($hasWmRuntime -ne $hasWmSource) -or ($hasWmRuntime -ne $hasWmVersion)) {
    throw '-WmRuntime, -WmSource and -WmVersion must be given together'
}

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$basePath = (Resolve-Path $Base).Path
$barPath = (Resolve-Path $Bar).Path
$binary = [System.IO.File]::ReadAllBytes($barPath)
$machineMap = @{ 'arm64' = 0xAA64; 'x64' = 0x8664 }

$baseArchive = [System.IO.Compression.ZipFile]::OpenRead($basePath)
try {
    $manifestBytes = Read-ZipEntryBytes -Archive $baseArchive -EntryName 'manifest.json'
    $manifestText = [System.Text.Encoding]::UTF8.GetString($manifestBytes).TrimStart([char]0xFEFF)
    $manifest = $manifestText | ConvertFrom-Json
    $arch = [string]$manifest.architecture

    foreach ($item in $manifest.files) {
        $entryBytes = Read-ZipEntryBytes -Archive $baseArchive -EntryName ([string]$item.path)
        if ((Get-BytesSha256 -Bytes $entryBytes) -ne ([string]$item.sha256).ToUpperInvariant()) {
            throw "Base integrity failed: $($item.path)"
        }
    }

    if ((Get-PeMachine -Bytes $binary) -ne $machineMap[$arch]) {
        throw 'Wrong binary architecture'
    }

    $folder = Join-Path $Output "GlazeBar-Community-$Version-windows-$arch"
    if (Test-Path $folder) {
        throw "Refusing to overwrite existing folder: $folder"
    }
    New-Item -ItemType Directory -Force -Path $folder | Out-Null
    [System.IO.Compression.ZipFileExtensions]::ExtractToDirectory($baseArchive, $folder)
} finally {
    $baseArchive.Dispose()
}

$tracked = Get-GitTrackedFiles -WorkingDir $repo
$trackedSet = New-Object 'System.Collections.Generic.HashSet[string]'
foreach ($t in $tracked) { [void]$trackedSet.Add($t) }

$sourceZip = Join-Path $folder 'source/glazebar-source.zip'
if ((-not $Slim) -and (Test-Path $sourceZip)) {
    $replacement = "$sourceZip.new.zip"
    if (Test-Path $replacement) { Remove-Item -Force $replacement }

    $oldArchive = [System.IO.Compression.ZipFile]::OpenRead($sourceZip)
    $newArchive = [System.IO.Compression.ZipFile]::Open($replacement, [System.IO.Compression.ZipArchiveMode]::Create)
    try {
        $oldLock = Normalize-Lockfile -Bytes (Read-ZipEntryBytes -Archive $oldArchive -EntryName 'native-rust/Cargo.lock')
        $localLock = Normalize-Lockfile -Bytes ([System.IO.File]::ReadAllBytes((Join-Path $repo 'native-rust/Cargo.lock')))
        if ($oldLock -ne $localLock) {
            throw 'Cannot reuse vendors with a different lockfile'
        }

        $written = New-Object 'System.Collections.Generic.HashSet[string]'
        $vendorSuffix = [System.Text.Encoding]::UTF8.GetBytes("`n[source.crates-io]`nreplace-with = 'vendored-sources'`n[source.vendored-sources]`ndirectory = 'vendor'`n")
        foreach ($entry in $oldArchive.Entries) {
            if ($entry.FullName.EndsWith('/')) { continue }
            $name = $entry.FullName
            $local = Join-Path $repo $name
            if ($name -eq 'native-rust/.cargo/config.toml') {
                $localBytes = [System.IO.File]::ReadAllBytes($local)
                $data = New-Object byte[] ($localBytes.Length + $vendorSuffix.Length)
                [System.Buffer]::BlockCopy($localBytes, 0, $data, 0, $localBytes.Length)
                [System.Buffer]::BlockCopy($vendorSuffix, 0, $data, $localBytes.Length, $vendorSuffix.Length)
            } elseif ($trackedSet.Contains($name)) {
                $data = [System.IO.File]::ReadAllBytes($local)
            } else {
                $data = Read-ZipEntryBytes -Archive $oldArchive -EntryName $name
            }
            Write-ZipEntryBytes -Archive $newArchive -EntryName $name -Bytes $data
            [void]$written.Add($name)
        }
        foreach ($name in $tracked) {
            if (-not $written.Contains($name)) {
                $local = Join-Path $repo $name
                if (Test-Path -LiteralPath $local -PathType Leaf) {
                    [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($newArchive, $local, $name, [System.IO.Compression.CompressionLevel]::Optimal) | Out-Null
                }
            }
        }
    } finally {
        $newArchive.Dispose()
        $oldArchive.Dispose()
    }
    Move-Item -Force $replacement $sourceZip
}

[System.IO.File]::WriteAllBytes((Join-Path $folder 'payload/GlazeBar.exe'), $binary)
$glazeThemePath = Join-Path $folder 'payload/GlazeTheme.exe'
if (Test-Path $glazeThemePath) {
    Remove-Item -Force $glazeThemePath
}

$iconsSrc = Join-Path $repo 'icons'
if (Test-Path $iconsSrc) {
    $iconsDst = Join-Path $folder 'payload/icons'
    if (Test-Path $iconsDst) {
        Remove-Item -Recurse -Force $iconsDst
    }
    Copy-Item -Recurse -Force $iconsSrc $iconsDst
}

foreach ($name in @('config.yaml', 'cheatsheet.tsv')) {
    Copy-Item -Force (Join-Path $repo "distribution/templates/$name") (Join-Path $folder "payload/$name")
}
foreach ($name in @('Install.ps1', 'Uninstall.ps1', 'README.txt', 'README.ko.txt')) {
    Copy-Item -Force (Join-Path $repo "distribution/$name") (Join-Path $folder $name)
}

$unchanged = @('modified GlazeWM', 'official GlazeWM MSI', 'dependency vendors', 'third-party licenses')
$wmProvenance = @{}
if ($hasWmRuntime) {
    $wmProvenance = Invoke-ReplaceModifiedWm -Folder $folder -Repo $repo -Arch $arch
    $manifest.glazewmVersion = $WmVersion
    $unchanged = $unchanged[1..($unchanged.Length - 1)]
}

$barRev = (& git -C $repo rev-parse HEAD).Trim()
$provenance = [ordered]@{
    version             = $Version
    architecture        = $arch
    baseBundle          = [System.IO.Path]::GetFileName($basePath)
    baseBundleSHA256    = Get-FileSha256 -Path $basePath
    barSourceRevision   = $barRev
    barSHA256           = Get-BytesSha256 -Bytes $binary
    glazewmVersion      = $manifest.glazewmVersion
    unchangedComponents = $unchanged
}
foreach ($key in $wmProvenance.Keys) {
    $provenance[$key] = $wmProvenance[$key]
}
Write-Utf8NoBom -Path (Join-Path $folder 'source/BUILD-GLAZEBAR.json') -Content ($provenance | ConvertTo-Json -Depth 10)

if ($Slim) {
    $sourceDir = Join-Path $folder 'source'
    if (Test-Path $sourceDir) {
        Get-ChildItem -Path $sourceDir -Filter '*.zip' -File | Remove-Item -Force
        Get-ChildItem -Path $sourceDir -Filter '*.tar*' -File | Remove-Item -Force
    }
    foreach ($comp in @('glazebar', 'glazewm')) {
        $compLicenses = Join-Path $folder "licenses/$comp"
        if (Test-Path $compLicenses) {
            Remove-Item -Recurse -Force $compLicenses
        }
    }
}

$manifest.version = $Version
$folderFull = (Resolve-Path $folder).Path.TrimEnd('\', '/')
$filesList = @(
    Get-ChildItem -Path $folderFull -Recurse -File |
        Where-Object { $_.Name -ne 'manifest.json' } |
        Sort-Object FullName |
        ForEach-Object {
            $rel = $_.FullName.Substring($folderFull.Length + 1).Replace('\', '/')
            [ordered]@{
                path   = $rel
                sha256 = Get-FileSha256 -Path $_.FullName
            }
        }
)
$manifest.files = $filesList
Write-Utf8NoBom -Path (Join-Path $folder 'manifest.json') -Content ($manifest | ConvertTo-Json -Depth 10)

$zipOut = "$folder.zip"
if (Test-Path $zipOut) { Remove-Item -Force $zipOut }
$outArchive = [System.IO.Compression.ZipFile]::Open($zipOut, [System.IO.Compression.ZipArchiveMode]::Create)
try {
    Get-ChildItem -Path $folderFull -Recurse -File | Sort-Object FullName | ForEach-Object {
        $rel = $_.FullName.Substring($folderFull.Length + 1).Replace('\', '/')
        [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($outArchive, $_.FullName, $rel, [System.IO.Compression.CompressionLevel]::Optimal) | Out-Null
    }
} finally {
    $outArchive.Dispose()
}

Write-Output $folder
