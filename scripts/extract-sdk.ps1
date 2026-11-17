<#
.SYNOPSIS
  Extracts the official Zoom Plugin SDK for Windows archive and activates it.

.DESCRIPTION
  Looks for vendor/downloads/zoom-plugin-sdk-windows-*.zip (newest wins) unless
  -Archive is given, extracts it under vendor/zoom-plugin-sdk/, and writes
  vendor/zoom-plugin-sdk/ACTIVE pointing at the chosen architecture tree
  (default x64). Both folders are git-ignored: SDK binaries are never committed.
  Re-running is idempotent.
#>
[CmdletBinding()]
param(
    [string]$Archive,
    [ValidateSet('x64', 'x86', 'arm64')][string]$Arch = 'x64',
    [switch]$Force
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$downloads = Join-Path $root 'vendor\downloads'
$sdkHome = Join-Path $root 'vendor\zoom-plugin-sdk'

if (-not $Archive) {
    $zip = Get-ChildItem $downloads -Filter 'zoom-plugin-sdk-windows-*.zip' -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $zip) {
        throw "No zoom-plugin-sdk-windows-*.zip in $downloads. Download it from Zoom App Marketplace > your General app > Features > Access > Plugin SDK > Windows."
    }
    $Archive = $zip.FullName
}
$name = [IO.Path]::GetFileNameWithoutExtension($Archive)
$hash = (Get-FileHash $Archive -Algorithm SHA256).Hash
Write-Host "Archive : $Archive"
Write-Host "SHA256  : $hash"

$dest = Join-Path $sdkHome $name
if ((Test-Path $dest) -and -not $Force) {
    Write-Host "Already extracted: $dest (use -Force to re-extract)"
} else {
    if (Test-Path $dest) { Remove-Item $dest -Recurse -Force }
    New-Item -ItemType Directory -Force $sdkHome | Out-Null
    # The archive contains a single top-level folder named like the zip.
    Expand-Archive -Path $Archive -DestinationPath $sdkHome -Force
    if (-not (Test-Path $dest)) { throw "Unexpected archive layout: $dest not created" }
}

$archRoot = Join-Path $dest $Arch
foreach ($p in 'export_h\ToolSuiteProxyInterface.h', 'lib\zToolSuiteIPCProxy.lib', 'bin\zToolSuiteIPCProxy.dll') {
    if (-not (Test-Path (Join-Path $archRoot $p))) { throw "Missing $p in $archRoot" }
}
Set-Content -Path (Join-Path $sdkHome 'ACTIVE') -Value $archRoot -Encoding utf8NoBOM -NoNewline
@{ archive = (Split-Path $Archive -Leaf); sha256 = $hash; arch = $Arch; root = $archRoot;
   version = (Get-Content (Join-Path $dest 'version.txt') -ErrorAction SilentlyContinue) -join ' ' } |
    ConvertTo-Json | Set-Content (Join-Path $sdkHome 'ACTIVE.json') -Encoding utf8NoBOM
Write-Host "Active SDK ($Arch): $archRoot"
