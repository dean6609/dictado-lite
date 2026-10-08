$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$isolatedCargo = Join-Path $projectRoot '.tools/cargo'
if (Test-Path (Join-Path $isolatedCargo 'bin/cargo.exe')) {
    $env:CARGO_HOME = $isolatedCargo
    $env:RUSTUP_HOME = Join-Path $projectRoot '.tools/rustup'
    $env:PATH = (Join-Path $isolatedCargo 'bin') + ';' + $env:PATH
}
