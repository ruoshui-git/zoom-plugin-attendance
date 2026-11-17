<#
.SYNOPSIS
  Dot-source to load the MSVC x64 developer environment (cl, link, dumpbin,
  MSBuild, VS-bundled CMake) into the current PowerShell session.
.EXAMPLE
  . .\scripts\dev-env.ps1
#>
. "$PSScriptRoot\common.ps1"
Import-VsDevEnv
$sdk = Get-ActiveSdkRoot
if ($sdk) { $env:ZOOM_PLUGIN_SDK_DIR = $sdk }
Write-Host "MSVC dev env loaded (VCToolsVersion=$env:VCToolsVersion). ZOOM_PLUGIN_SDK_DIR=$env:ZOOM_PLUGIN_SDK_DIR"
