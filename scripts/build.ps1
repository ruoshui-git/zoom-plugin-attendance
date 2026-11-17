<#
.SYNOPSIS
  Builds the Rust workspace (debug by default) and copies the complete SDK
  runtime (bin\*) next to target\<profile>\zoom-companion.exe.
  Verified: the SDK only delivers auth/IPC callbacks when its runtime set sits
  beside the executable.
#>
[CmdletBinding()]
param([switch]$Release)
. "$PSScriptRoot\common.ps1"
Push-Location $script:RepoRoot
try {
    $uiDir = Join-Path $script:RepoRoot 'apps\desktop-ui'
    Push-Location $uiDir
    try {
        pnpm install --frozen-lockfile
        if ($LASTEXITCODE -ne 0) { throw "pnpm install failed ($LASTEXITCODE)" }
        pnpm build
        if ($LASTEXITCODE -ne 0) { throw "UI build failed ($LASTEXITCODE)" }
    } finally { Pop-Location }
    $a = @('build', '--workspace')
    if ($Release) { $a += '--release' }
    cargo @a
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
    $profile = if ($Release) { 'release' } else { 'debug' }
    $outDir = Join-Path $script:RepoRoot ("target\" + $profile)
    $uiOut = Join-Path $outDir 'ui'
    if (Test-Path $uiOut) { Remove-Item $uiOut -Recurse -Force }
    Copy-Item (Join-Path $uiDir 'dist') $uiOut -Recurse
    Write-Host "Desktop UI copied to $uiOut"
    $sdk = Get-ActiveSdkRoot
    if ($sdk) {
        Get-ChildItem (Join-Path $sdk 'bin') -Exclude 'PSDKTest.*' | ForEach-Object {
            $dest = Join-Path $outDir $_.Name
            if (-not (Test-Path $dest) -or (Get-Item $dest).Length -ne $_.Length) { Copy-Item $_.FullName $dest -Recurse -Force }
        }
        Write-Host "SDK runtime synced to $outDir"
    } else {
        Write-Warning 'No active SDK: built in stub mode.'
    }
} finally { Pop-Location }
