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
    $a = @('build', '--workspace')
    if ($Release) { $a += '--release' }
    cargo @a
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
    $sdk = Get-ActiveSdkRoot
    if ($sdk) {
        $outDir = Join-Path $script:RepoRoot ("target\" + $(if ($Release) { 'release' } else { 'debug' }))
        Get-ChildItem (Join-Path $sdk 'bin') -Exclude 'PSDKTest.*' | ForEach-Object {
            $dest = Join-Path $outDir $_.Name
            if (-not (Test-Path $dest) -or (Get-Item $dest).Length -ne $_.Length) { Copy-Item $_.FullName $dest -Recurse -Force }
        }
        Write-Host "SDK runtime synced to $outDir"
    } else {
        Write-Warning 'No active SDK: built in stub mode.'
    }
} finally { Pop-Location }
