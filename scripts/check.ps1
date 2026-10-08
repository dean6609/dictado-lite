$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'dev-env.ps1')
if (!$env:TRANSCRIBE_DIR) { throw 'First run scripts/build-native.ps1 in the same shell' }
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    cargo fmt --all --check
    if ($LASTEXITCODE -ne 0) { throw 'Formatting failed' }
    cargo clippy --locked --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Clippy failed' }
    & (Join-Path $PSScriptRoot 'stage-runtime.ps1') -Directory 'target/debug/deps'
    cargo test --locked
    if ($LASTEXITCODE -ne 0) { throw 'Tests failed' }
    cargo build --locked --release
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
    & (Join-Path $PSScriptRoot 'stage-runtime.ps1')
} finally { Pop-Location }
