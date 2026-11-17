<#
.SYNOPSIS
  Builds the official PSDKTest sample (MFC) from the active SDK, x64.

.DESCRIPTION
  The sample targets toolset v142 (VS2019). This machine has VS 2022 Build
  Tools (v143), so the toolset is overridden on the command line; the SDK files
  themselves are not modified. Output (per the official project) goes to the
  SDK's <arch>\bin\PSDKTest.exe, next to the required runtime DLLs.
  Requires the "C++ ATL/MFC" component (Microsoft.VisualStudio.Component.VC.ATLMFC).
#>
[CmdletBinding()]
param(
    [ValidateSet('Release', 'Debug')][string]$Configuration = 'Release',
    [string]$Toolset = 'v143',
    [switch]$Launch
)
. "$PSScriptRoot\common.ps1"

$sdk = Get-ActiveSdkRoot
if (-not $sdk) { throw 'No active SDK. Run scripts\extract-sdk.ps1 first.' }
$sln = Join-Path $sdk 'demo\PSDKTest.sln'
if (-not (Test-Path $sln)) { throw "Sample solution not found: $sln" }

$msbuild = Get-MSBuildPath
Write-Host "MSBuild : $msbuild"
Write-Host "Solution: $sln ($Configuration|x64, toolset $Toolset)"
& $msbuild $sln /t:Build /m /nologo /v:minimal `
    "/p:Configuration=$Configuration" /p:Platform=x64 `
    "/p:PlatformToolset=$Toolset" /p:WindowsTargetPlatformVersion=10.0
if ($LASTEXITCODE -ne 0) { throw "PSDKTest build failed ($LASTEXITCODE)" }

$exe = Join-Path $sdk 'bin\PSDKTest.exe'
if (-not (Test-Path $exe)) { throw "Build reported success but $exe is missing" }
Write-Host "Built: $exe"
if ($Launch) { Start-Process -FilePath $exe -WorkingDirectory (Split-Path $exe) }
