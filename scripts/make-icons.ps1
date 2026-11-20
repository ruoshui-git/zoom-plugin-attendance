$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$assets = Join-Path $repoRoot 'packaging\assets'
$msixAssets = Join-Path $repoRoot 'packaging\msix\Assets'
$source = Join-Path $assets 'icon-source.png'
if (-not (Get-Command magick -ErrorAction SilentlyContinue)) { throw 'ImageMagick 7 (magick) is required.' }
if (-not (Test-Path $source)) { throw "Icon source not found: $source" }
& magick $source -define icon:auto-resize=256,64,48,40,32,24,20,16 (Join-Path $assets 'app.ico')
if ($LASTEXITCODE -ne 0) { throw "Failed generating app.ico ($LASTEXITCODE)" }
$variants = @(
    @{ Name = 'Square150x150Logo.png'; Size = 150 },
    @{ Name = 'Square44x44Logo.png'; Size = 44 },
    @{ Name = 'Square44x44Logo.targetsize-256_altform-unplated.png'; Size = 256 },
    @{ Name = 'StoreLogo.png'; Size = 50 }
)
foreach ($variant in $variants) {
    & magick $source -resize "$($variant.Size)x$($variant.Size)!" (Join-Path $msixAssets $variant.Name)
    if ($LASTEXITCODE -ne 0) { throw "Failed generating $($variant.Name) ($LASTEXITCODE)" }
}
