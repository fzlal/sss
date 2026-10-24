<#
.SYNOPSIS
    Build the portable Windows zip and, unless -SkipInstaller is set, the NSIS
    setup executable.

.EXAMPLE
    ./packaging/windows/package.ps1 -Version 0.1.0 -Arch x86_64 `
        -Binary target/x86_64-pc-windows-msvc/release/slime-soccer.exe `
        -Dist dist
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Version,
    [Parameter(Mandatory = $true)][ValidateSet('x86_64', 'i686', 'aarch64')][string]$Arch,
    [Parameter(Mandatory = $true)][string]$Binary,
    [Parameter(Mandatory = $true)][string]$Dist,
    [switch]$SkipInstaller
)

$ErrorActionPreference = 'Stop'

$Version = $Version.TrimStart('v')
if ($Version -notmatch '^[0-9A-Za-z.+~-]+$') {
    throw "Invalid version: $Version"
}

$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$Binary = (Resolve-Path $Binary).Path
if (-not [System.IO.Path]::IsPathRooted($Dist)) {
    $Dist = Join-Path (Get-Location).Path $Dist
}
$Dist = [System.IO.Path]::GetFullPath($Dist)

$stage = Join-Path ([System.IO.Path]::GetTempPath()) ("slime-soccer-" + [Guid]::NewGuid().ToString('N'))
$portableStage = Join-Path ([System.IO.Path]::GetTempPath()) ("slime-soccer-portable-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $stage | Out-Null
New-Item -ItemType Directory -Force -Path $portableStage | Out-Null
New-Item -ItemType Directory -Force -Path $Dist | Out-Null

try {
    Copy-Item $Binary (Join-Path $stage 'slime-soccer.exe')
    foreach ($doc in @('README.md', 'LICENSE', 'ASSETS.md', 'THIRD_PARTY_NOTICES.md')) {
        $source = Join-Path $Root $doc
        if (-not (Test-Path $source)) {
            throw "missing required file: $source"
        }
        Copy-Item $source (Join-Path $stage $doc)
    }

    # The portable zip stores saves next to the executable; the installer must
    # not contain the marker or uninstalling would delete the player's progress.
    Copy-Item -Path (Join-Path $stage '*') -Destination $portableStage -Recurse
    Copy-Item (Join-Path $Root 'packaging\windows\portable.txt') (Join-Path $portableStage 'portable.txt')

    $zip = Join-Path $Dist "slime-soccer-$Version-windows-$Arch.zip"
    if (Test-Path $zip) { Remove-Item -Force $zip }
    Compress-Archive -Path (Join-Path $portableStage '*') -DestinationPath $zip -CompressionLevel Optimal
    Write-Host "built $zip"

    if (-not $SkipInstaller) {
        $makensis = $null
        $command = Get-Command makensis -ErrorAction SilentlyContinue
        if ($command) {
            $makensis = $command.Source
        } else {
            $candidates = @()
            $programFilesX86 = [Environment]::GetFolderPath('ProgramFilesX86')
            if ($programFilesX86) {
                $candidates += Join-Path $programFilesX86 'NSIS\makensis.exe'
            }
            if ($env:ProgramFiles) {
                $candidates += Join-Path $env:ProgramFiles 'NSIS\makensis.exe'
            }
            $candidates += 'C:\ProgramData\chocolatey\bin\makensis.exe'
            foreach ($candidate in $candidates) {
                if (Test-Path $candidate) {
                    $makensis = $candidate
                    break
                }
            }
        }
        if (-not $makensis) {
            throw 'makensis was not found; install NSIS or pass -SkipInstaller'
        }

        $installer = Join-Path $Dist "slime-soccer-$Version-windows-$Arch-setup.exe"
        $icon = Join-Path $Root 'packaging\icons\slime-soccer.ico'
        $nsisArgs = @(
            '/V2',
            "-DAPP_VERSION=$Version",
            "-DAPP_ARCH=$Arch",
            "-DSTAGE=$stage",
            "-DOUT_FILE=$installer",
            "-DICON_PATH=$icon",
            (Join-Path $PSScriptRoot 'installer.nsi')
        )
        & $makensis @nsisArgs
        if ($LASTEXITCODE -ne 0) {
            throw "makensis failed with exit code $LASTEXITCODE"
        }
        if (-not (Test-Path $installer)) {
            throw "NSIS did not create $installer"
        }
        Write-Host "built $installer"
    }
} finally {
    Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force $portableStage -ErrorAction SilentlyContinue
}

Write-Host "Windows packages in $Dist"
