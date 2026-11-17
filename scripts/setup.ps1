<#
.SYNOPSIS
  Idempotent prerequisite check/installer for this repository (Windows x64).

.DESCRIPTION
  Inspects first; installs only what is missing:
    * rustup + stable-x86_64-pc-windows-msvc       (winget Rustlang.Rustup)
    * Visual Studio 2022 Build Tools with components:
        VC.Tools.x86.x64, Windows11SDK.26100, VC.ATLMFC (for the MFC PSDKTest sample)
      An existing instance is MODIFIED (no second VS install). Requires admin.
    * Git                                           (winget Git.Git)
  CMake is not required by the build (cc crate + MSBuild); the VS-bundled
  CMake is available after scripts\dev-env.ps1.

  Note: if Defender Controlled Folder Access protects the VS install folder,
  the VS Installer must be an allowed app (see docs/SETUP.md).
#>
[CmdletBinding()]
param([switch]$CheckOnly)
. "$PSScriptRoot\common.ps1"

function Have($cmd) { [bool](Get-Command $cmd -ErrorAction SilentlyContinue) }
$missing = @()

# Rust
if (-not (Have rustup)) {
    if ($CheckOnly) { $missing += 'rustup' } else { winget install --id Rustlang.Rustup -e --accept-package-agreements --accept-source-agreements }
} else {
    $tc = rustup show active-toolchain 2>$null
    Write-Host "rust     : $tc"
    if ($tc -notmatch 'x86_64-pc-windows-msvc') {
        if ($CheckOnly) { $missing += 'msvc toolchain' } else { rustup default stable-x86_64-pc-windows-msvc }
    }
}

# Git
if (Have git) { Write-Host "git      : $(git --version)" } elseif ($CheckOnly) { $missing += 'git' } else { winget install --id Git.Git -e }

# Visual Studio Build Tools + components
$components = @(
    'Microsoft.VisualStudio.Component.VC.Tools.x86.x64',
    'Microsoft.VisualStudio.Component.Windows11SDK.26100',
    'Microsoft.VisualStudio.Component.VC.ATLMFC'
)
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$inst = if (Test-Path $vswhere) { & $vswhere -latest -products * -format json | ConvertFrom-Json | Select-Object -First 1 } else { $null }
if (-not $inst) {
    if ($CheckOnly) { $missing += 'Visual Studio 2022 Build Tools' }
    else {
        $add = ($components | ForEach-Object { "--add $_" }) -join ' '
        winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--quiet --wait --norestart $add"
    }
} else {
    Write-Host "vs       : $($inst.displayName) $($inst.installationVersion) @ $($inst.installationPath)"
    $need = $components | Where-Object { -not (& $vswhere -path $inst.installationPath -requires $_ -property instanceId) }
    if ($need) {
        if ($CheckOnly) { $missing += $need }
        else {
            $setup = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\setup.exe'
            $args = @('modify', '--installPath', "`"$($inst.installationPath)`"", '--quiet', '--norestart') + ($need | ForEach-Object { '--add', $_ })
            $p = Start-Process $setup -ArgumentList $args -Wait -PassThru
            if ($p.ExitCode -ne 0) { throw "VS Installer exited $($p.ExitCode) (admin rights? Controlled Folder Access?)" }
        }
    } else { Write-Host 'vs comps : all required components present' }
}

# Zoom Workplace (informational; installed by the user, not by this script)
$zoom = Join-Path $env:APPDATA 'Zoom\bin\Zoom.exe'
if (Test-Path $zoom) { Write-Host "zoom     : Zoom Workplace $((Get-Item $zoom).VersionInfo.ProductVersion)" }
else { Write-Warning 'Zoom Workplace not found in %APPDATA%\Zoom\bin (needs >= 7.0.2)' }

# SDK
$sdk = Get-ActiveSdkRoot
if ($sdk) { Write-Host "zoom sdk : $sdk" } else { Write-Warning 'Zoom Plugin SDK not extracted: put the zip in vendor\downloads and run scripts\extract-sdk.ps1' }

if ($missing) { Write-Warning "Missing: $($missing -join ', ')"; exit 1 }
Write-Host 'setup: OK'
