$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'dev-env.ps1')
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    cargo fmt --all --check
    if ($LASTEXITCODE -ne 0) { throw 'Formatting failed' }
    cargo clippy --locked --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Clippy failed' }
    cargo test --locked
    if ($LASTEXITCODE -ne 0) { throw 'Tests failed' }
    cargo build --locked --release
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
} finally { Pop-Location }
