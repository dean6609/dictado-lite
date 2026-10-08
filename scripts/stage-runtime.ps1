param([string]$Directory = 'target/release')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
if (!$env:TRANSCRIBE_DIR) { throw 'Run scripts/build-native.ps1 in this shell first' }
$destination = Join-Path $projectRoot $Directory
New-Item -ItemType Directory -Force $destination | Out-Null
Copy-Item (Join-Path $env:TRANSCRIBE_DIR 'bin/*.dll') $destination -Force
if ($env:DICTADO_GNU -eq '1') {
    $runtimeRoot = Join-Path $projectRoot '.tools/llvm-mingw-20260922-ucrt-x86_64/x86_64-w64-mingw32/bin'
    foreach ($file in 'libc++.dll','libunwind.dll') { Copy-Item (Join-Path $runtimeRoot $file) $destination -Force }
}
