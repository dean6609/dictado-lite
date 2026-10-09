param([string]$Installer)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
if (!$Installer) {
    $Installer = (Get-ChildItem (Join-Path $projectRoot 'artifacts/*-Setup.exe') | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
}
if (!$Installer) { throw 'Build the installer first' }
$qaRoot = Join-Path $env:LOCALAPPDATA 'Programs/DictadoLite-QA'
$fixture = Join-Path $qaRoot 'preserve-smoke.txt'
if (Test-Path -LiteralPath $fixture) { throw 'Refusing to replace an existing QA fixture' }
function Invoke-Setup {
    param([string]$Path, [string[]]$Arguments)
    $process = Start-Process -FilePath $Path -ArgumentList $Arguments -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "Setup failed: $($process.ExitCode)" }
}
Invoke-Setup $Installer @('--quiet','--test')
$manifest = Get-Content (Join-Path $qaRoot 'installed-files.json') -Raw | ConvertFrom-Json
if ($manifest.files.name -match '\.gguf$') { throw 'Installer must not bundle weights' }
foreach ($name in 'NOTICE','LICENSE','THIRD_PARTY_NOTICES.md','models/catalog.json','licenses/miniz-LICENSE.txt') {
    if (!(Test-Path -LiteralPath (Join-Path $qaRoot $name))) { throw "Missing notice/metadata: $name" }
}
Set-Content -LiteralPath $fixture -Value 'installer-smoke-owned-fixture' -Encoding ascii
try {
    Invoke-Setup $Installer @('--quiet','--test')
    if ((Get-Content -LiteralPath $fixture -Raw).Trim() -ne 'installer-smoke-owned-fixture') { throw 'Upgrade failed to preserve unowned content' }
    Invoke-Setup (Join-Path $projectRoot 'target/release/dictado-setup.exe') @('--remove','--quiet','--test')
    if (!(Test-Path -LiteralPath $fixture)) { throw 'Uninstall deleted unowned content' }
    if (Test-Path (Join-Path $qaRoot 'dictado-lite.exe')) { throw 'Uninstall left an owned application file' }
} finally {
    # Remove only the fixture created above, never recursively delete the QA root.
    if ((Test-Path -LiteralPath $fixture) -and (Get-Content -LiteralPath $fixture -Raw).Trim() -eq 'installer-smoke-owned-fixture') {
        Remove-Item -LiteralPath $fixture
    }
}
Write-Output 'Light installer: install, upgrade, notices, no weights, uninstall and preservation passed.'
