<#
.SYNOPSIS  Builds (incl. SDK runtime sync) and runs zoom-companion with the given arguments.
.EXAMPLE   .\scripts\run.ps1 doctor
.EXAMPLE   .\scripts\run.ps1 connect --seconds 30
#>
. "$PSScriptRoot\common.ps1"
& "$PSScriptRoot\build.ps1" *> $null
if ($LASTEXITCODE) { & "$PSScriptRoot\build.ps1"; exit 1 }
& (Join-Path $script:RepoRoot 'target\debug\zoom-companion.exe') @args
exit $LASTEXITCODE
