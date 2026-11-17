<#
.SYNOPSIS
  Release build + self-contained folder: dist\zoom-companion\ with the exe and
  the COMPLETE SDK bin\ runtime set beside it (official requirement).
  dist\ is git-ignored. Do not redistribute without reviewing Zoom's SDK terms.
#>
[CmdletBinding()]
param()
. "$PSScriptRoot\common.ps1"
$sdk = Get-ActiveSdkRoot
if (-not $sdk) { throw 'No active SDK (scripts\extract-sdk.ps1).' }
& "$PSScriptRoot\build.ps1" -Release
$out = Join-Path $script:RepoRoot 'dist\zoom-companion'
if (Test-Path $out) { Remove-Item $out -Recurse -Force }
New-Item -ItemType Directory -Force $out | Out-Null
Copy-Item (Join-Path $sdk 'bin\*') $out -Recurse -Exclude 'PSDKTest.*'
Copy-Item (Join-Path $script:RepoRoot 'target\release\zoom-companion.exe') $out
Write-Host "Packaged: $out"
