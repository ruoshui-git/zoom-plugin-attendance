<#
.SYNOPSIS
  Release build + distributable packages for AttendFlow Desktop.
.DESCRIPTION
  Produces, under dist\ (git-ignored):
    AttendFlowDesktop\                     self-contained folder: AttendFlowDesktop.exe (launcher),
                                           zoom-companion.exe, ui\, and the COMPLETE SDK bin\ set
                                           beside the exe (official requirement)
    AttendFlowDesktop-<ver>-x64.zip        portable zip of that folder (beta testers)
    AttendFlowDesktop-<ver>-x64.msix       with -Msix: unsigned MSIX for the Microsoft Store
                                           (the Store signs it on publication)
    msix-layout\                           with -Msix: the unpacked package, for a local test:
                                           Add-AppxPackage -Register dist\msix-layout\AppxManifest.xml
                                           (needs Windows Developer Mode)

  The Marketplace PUBLIC client ID is baked into the build so users never configure
  anything. It defaults to the PRODUCTION public client ID (anything given to other
  people or to Zoom's reviewers must use it; development credentials only work for
  your own Zoom account). A public PKCE client ID is not a secret.
  -ClientId <dev public client id> builds a development package instead.
.EXAMPLE
  .\scripts\package.ps1 -Msix `
      -IdentityName 12345AttendFlow.AttendFlowDesktop -Publisher 'CN=XXXXXXXX-...' -PublisherDisplayName 'AttendFlow'
#>
[CmdletBinding()]
param(
    # Production "Public client ID" (Marketplace > AttendFlow Desktop > Production credentials).
    [string]$ClientId = '_up3N0AUTXmUr5u0F74umg',
    [switch]$Msix,
    # From Partner Center > Product identity (placeholders are fine for local testing).
    [string]$IdentityName = 'AttendFlow.Desktop.Dev',
    [string]$Publisher = 'CN=AttendFlow Dev',
    [string]$PublisherDisplayName = 'AttendFlow',
    # Build without a built-in client ID (then every user would need `config init`).
    [switch]$AllowNoClientId
)
. "$PSScriptRoot\common.ps1"

if ($AllowNoClientId) { $ClientId = '' }
elseif (-not $ClientId) {
    throw 'Pass -ClientId <Marketplace public client ID> (or -AllowNoClientId for a developer-only build).'
}
$sdk = Get-ActiveSdkRoot
if (-not $sdk) { throw 'No active SDK (scripts\extract-sdk.ps1).' }

$cargoToml = Get-Content (Join-Path $script:RepoRoot 'Cargo.toml') -Raw
if ($cargoToml -notmatch '(?m)^version\s*=\s*"(\d+)\.(\d+)\.(\d+)"') { throw 'workspace version not found in Cargo.toml' }
$version = "$($Matches[1]).$($Matches[2]).$($Matches[3])"
$msixVersion = "$version.0"   # Store requires the 4th part to be 0

# option_env! in crates/zoom-companion/src/config.rs reads this at compile time;
# cargo rebuilds when it changes.
$prevClientId = $env:ZOOM_COMPANION_BUILTIN_CLIENT_ID
$env:ZOOM_COMPANION_BUILTIN_CLIENT_ID = $ClientId
try {
    & "$PSScriptRoot\build.ps1" -Release
} finally {
    $env:ZOOM_COMPANION_BUILTIN_CLIENT_ID = $prevClientId
}

$dist = Join-Path $script:RepoRoot 'dist'
$out = Join-Path $dist 'AttendFlowDesktop'
if (Test-Path $out) { Remove-Item $out -Recurse -Force }
New-Item -ItemType Directory -Force $out | Out-Null
Copy-Item (Join-Path $sdk 'bin\*') $out -Recurse -Exclude 'PSDKTest.*'
$rel = Join-Path $script:RepoRoot 'target\release'
Copy-Item (Join-Path $rel 'zoom-companion.exe') $out
Copy-Item (Join-Path $rel 'AttendFlowDesktop.exe') $out
Copy-Item (Join-Path $rel 'ui') (Join-Path $out 'ui') -Recurse
Write-Host "Packaged folder: $out"
Write-Host ("Built-in client ID: " + $(if ($ClientId) { $ClientId } else { '<none: developer build>' }))

$zip = Join-Path $dist "AttendFlowDesktop-$version-x64.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path $out -DestinationPath $zip
Write-Host "Zip: $zip"

if ($Msix) {
    $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $kitBin = Get-ChildItem $kits -Directory |
        Where-Object { Test-Path (Join-Path $_.FullName 'x64\makeappx.exe') } |
        Sort-Object { [version]$_.Name } | Select-Object -Last 1
    if (-not $kitBin) { throw "makeappx.exe not found under $kits (install the Windows SDK)" }
    $makeappx = Join-Path $kitBin.FullName 'x64\makeappx.exe'
    $makepri = Join-Path $kitBin.FullName 'x64\makepri.exe'

    $layout = Join-Path $dist 'msix-layout'
    if (Test-Path $layout) { Remove-Item $layout -Recurse -Force }
    Copy-Item $out $layout -Recurse
    Copy-Item (Join-Path $script:RepoRoot 'packaging\msix\Assets') (Join-Path $layout 'Assets') -Recurse
    Copy-Item (Join-Path $script:RepoRoot 'packaging\msix\Strings') (Join-Path $layout 'Strings') -Recurse
    $manifest = (Get-Content (Join-Path $script:RepoRoot 'packaging\msix\AppxManifest.template.xml') -Raw).
        Replace('{{IdentityName}}', $IdentityName).
        Replace('{{Publisher}}', [Security.SecurityElement]::Escape($Publisher)).
        Replace('{{PublisherDisplayName}}', [Security.SecurityElement]::Escape($PublisherDisplayName)).
        Replace('{{Version}}', $msixVersion)
    Set-Content -Path (Join-Path $layout 'AppxManifest.xml') -Value $manifest -Encoding utf8

    # resources.pri lets Windows pick the unplated 256 px taskbar icon variant.
    $priCfg = Join-Path $dist 'priconfig.xml'
    & $makepri createconfig /cf $priCfg /dq en-US_zh-CN /o | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "makepri createconfig failed ($LASTEXITCODE)" }
    & $makepri new /pr $layout /cf $priCfg /of (Join-Path $layout 'resources.pri') /o | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "makepri new failed ($LASTEXITCODE)" }
    Remove-Item $priCfg

    $msixFile = Join-Path $dist "AttendFlowDesktop-$version-x64.msix"
    & $makeappx pack /d $layout /p $msixFile /o | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "makeappx pack failed ($LASTEXITCODE)" }
    Write-Host "MSIX (unsigned, identity $IdentityName / $Publisher): $msixFile"
    Write-Host "Local test (Developer Mode): Add-AppxPackage -Register `"$layout\AppxManifest.xml`""
}
