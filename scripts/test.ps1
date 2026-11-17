<#
.SYNOPSIS  Runs unit tests and the interop self-test. No Zoom account needed.
#>
[CmdletBinding()]
param()
. "$PSScriptRoot\common.ps1"
Push-Location $script:RepoRoot
try {
    cargo test --workspace
    if ($LASTEXITCODE -ne 0) { throw "cargo test failed ($LASTEXITCODE)" }
    cargo run -q -p zoom-companion -- selftest
    if ($LASTEXITCODE -ne 0) { throw "selftest failed ($LASTEXITCODE)" }
} finally { Pop-Location }
