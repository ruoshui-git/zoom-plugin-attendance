# Shared helpers for the scripts in this folder. Dot-source: . "$PSScriptRoot\common.ps1"
$ErrorActionPreference = 'Stop'
$script:RepoRoot = Split-Path -Parent $PSScriptRoot

function Get-VsInstallPath {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path $vswhere)) { throw 'vswhere.exe not found: install Visual Studio 2022 Build Tools.' }
    $p = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if (-not $p) { throw 'No Visual Studio instance with the MSVC x64 toolset was found.' }
    return $p
}

function Get-MSBuildPath {
    $p = Join-Path (Get-VsInstallPath) 'MSBuild\Current\Bin\MSBuild.exe'
    if (-not (Test-Path $p)) { throw "MSBuild not found at $p" }
    return $p
}

# Imports the vcvars64 environment (cl, link, cmake, ninja on PATH) into this session.
function Import-VsDevEnv {
    if ($env:VSCMD_ARG_TGT_ARCH -eq 'x64') { return }
    $bat = Join-Path (Get-VsInstallPath) 'VC\Auxiliary\Build\vcvars64.bat'
    $lines = cmd /c "`"$bat`" >nul 2>&1 && set"
    foreach ($l in $lines) {
        if ($l -match '^([^=]+)=(.*)$') { Set-Item -Path "env:$($Matches[1])" -Value $Matches[2] }
    }
}

function Get-ActiveSdkRoot {
    if ($env:ZOOM_PLUGIN_SDK_DIR) { return $env:ZOOM_PLUGIN_SDK_DIR }
    $f = Join-Path $script:RepoRoot 'vendor\zoom-plugin-sdk\ACTIVE'
    if (Test-Path $f) { return (Get-Content $f -Raw).Trim() }
    return $null
}
