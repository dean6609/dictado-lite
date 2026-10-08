$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'dev-env.ps1')
$projectRoot = Split-Path $PSScriptRoot -Parent
$manifest = Get-Content (Join-Path $projectRoot 'models/manifest.json') -Raw | ConvertFrom-Json
if (!$env:DICTADO_TEST_MODEL) {
    $modelRoot = Join-Path $projectRoot '.tools/models'
    New-Item -ItemType Directory -Force $modelRoot | Out-Null
    $env:DICTADO_TEST_MODEL = Join-Path $modelRoot $manifest.filename
    if (!(Test-Path $env:DICTADO_TEST_MODEL)) {
        Invoke-WebRequest "$($manifest.conversion_repository)/resolve/$($manifest.revision)/$($manifest.filename)" -OutFile $env:DICTADO_TEST_MODEL
    }
}
if ((Get-Item $env:DICTADO_TEST_MODEL).Length -ne $manifest.bytes -or (Get-FileHash $env:DICTADO_TEST_MODEL).Hash -ne $manifest.sha256) {
    throw 'Smoke model integrity failed'
}
Push-Location $projectRoot
try {
    & (Join-Path $PSScriptRoot 'stage-runtime.ps1') -Directory 'target/release/deps'
    cargo test --locked --release --test native_smoke -- --ignored
    if ($LASTEXITCODE -ne 0) { throw 'Native smoke failed' }
} finally { Pop-Location }
