param([switch]$Vulkan, [switch]$Gnu)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$toolsRoot = Join-Path $projectRoot '.tools'
New-Item -ItemType Directory -Force $toolsRoot | Out-Null
$sources = Get-Content (Join-Path $PSScriptRoot 'tool-sources.json') -Raw | ConvertFrom-Json
foreach ($source in $sources) {
    if ($source.name -eq 'vulkan' -and !$Vulkan) { continue }
    if ($source.name -eq 'llvm-mingw' -and !$Gnu) { continue }
    if (Test-Path (Join-Path $toolsRoot $source.marker)) { continue }
    $archive = Join-Path $toolsRoot $source.file
    if (!(Test-Path $archive)) { Invoke-WebRequest $source.url -OutFile $archive }
    if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $source.sha256) {
        throw "Hash mismatch for $($source.name)"
    }
    $destination = Join-Path $toolsRoot $source.destination
    if ($source.name -eq 'vulkan') {
        & $archive --root $destination --accept-licenses --default-answer --confirm-command install copy_only=1
        if ($LASTEXITCODE -ne 0) { throw 'Vulkan SDK extraction failed' }
    } else { Expand-Archive -LiteralPath $archive -DestinationPath $destination -Force }
}
