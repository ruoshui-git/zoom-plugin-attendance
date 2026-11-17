param(
  [Parameter(Mandatory=$true)][string]$AttendFlowRoot
)
$ErrorActionPreference = 'Stop'
$source = Join-Path $PSScriptRoot 'replay.test.ts'
$target = Join-Path $AttendFlowRoot 'src/components/meeting/__tests__/attend-engine-generate.test.ts'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../../../../../..')).Path
$fixtureRoot = Join-Path $PSScriptRoot '../fixtures'
$goldenRoot = Join-Path $PSScriptRoot '../goldens'
New-Item -ItemType Directory -Force $goldenRoot | Out-Null
Copy-Item -LiteralPath $source -Destination $target -Force
try {
  $env:ATTEND_ENGINE_FIXTURES = (Resolve-Path $fixtureRoot).Path
  $env:ATTEND_ENGINE_GOLDENS = (Resolve-Path $goldenRoot).Path
  Push-Location $AttendFlowRoot
  try { & npx vitest run src/components/meeting/__tests__/attend-engine-generate.test.ts --reporter=dot --silent; if ($LASTEXITCODE -ne 0) { throw "Vitest exited $LASTEXITCODE" } }
  finally { Pop-Location }
}
finally { Remove-Item -LiteralPath $target -Force -ErrorAction SilentlyContinue }
