param([switch]$Vulkan, [switch]$Gnu)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'dev-env.ps1')
$projectRoot = Split-Path $PSScriptRoot -Parent
$toolsRoot = Join-Path $projectRoot '.tools'
& (Join-Path $PSScriptRoot 'bootstrap-tools.ps1') -Vulkan:$Vulkan -Gnu:$Gnu
$env:PATH = (Join-Path $toolsRoot 'cmake-4.3.5-windows-x86_64/bin') + ';' + (Join-Path $toolsRoot 'ninja') + ';' + $env:PATH
if ($Gnu) {
    $llvmRoot = Join-Path $toolsRoot 'llvm-mingw-20260922-ucrt-x86_64'
    $env:PATH = (Join-Path $llvmRoot 'bin') + ';' + $env:PATH
    $sysroot = (rustc --print sysroot).Trim()
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = Join-Path $sysroot 'lib/rustlib/x86_64-pc-windows-gnu/bin/gcc-ld/ld.lld.exe'
    $env:RUSTFLAGS = '-C linker-flavor=ld -C link-self-contained=yes'
}
if ($Vulkan) { $env:VULKAN_SDK = Join-Path $toolsRoot 'vulkan' }
$sourceRoot = Join-Path $toolsRoot 'parakeet-source/transcribe-cpp-sys-0.2.4'
if (!(Test-Path $sourceRoot)) {
    $archive = Join-Path $toolsRoot 'transcribe-cpp-sys-0.2.4.crate'
    if (!(Test-Path $archive)) { Invoke-WebRequest 'https://crates.io/api/v1/crates/transcribe-cpp-sys/0.2.4/download' -OutFile $archive }
    if ((Get-FileHash $archive).Hash -ne '1c6946c7bf90046fc51e5bc25373a4f6270ca10fdcf4f35b1174075e43c535b5') { throw 'Native source hash mismatch' }
    New-Item -ItemType Directory -Force (Split-Path $sourceRoot -Parent) | Out-Null
    tar -xzf $archive -C (Split-Path $sourceRoot -Parent)
    if ($LASTEXITCODE -ne 0) { throw 'Native source extraction failed' }
}
& (Join-Path $PSScriptRoot 'prepare-parakeet.ps1') -SourceRoot $sourceRoot
$flavor = if ($Gnu) { 'gnu' } else { 'msvc' }
$gpu = if ($Vulkan) { 'vulkan' } else { 'cpu' }
$buildRoot = Join-Path $toolsRoot "parakeet-build-$flavor-$gpu"
$prefix = Join-Path $toolsRoot "parakeet-$flavor-$gpu"
$arguments = @('-S', $sourceRoot, '-B', $buildRoot, '-DCMAKE_BUILD_TYPE=Release', '-DTRANSCRIBE_INSTALL=ON', '-DTRANSCRIBE_BUILD_SHARED=ON', '-DTRANSCRIBE_GGML_BACKEND_DL=ON', '-DTRANSCRIBE_BUILD_TESTS=OFF', '-DTRANSCRIBE_BUILD_EXAMPLES=OFF', '-DTRANSCRIBE_BUILD_TOOLS=OFF', '-DTRANSCRIBE_USE_SYSTEM_BLAS=OFF', '-DTRANSCRIBE_X86_CONSERVATIVE=ON', '-DGGML_CPU_ALL_VARIANTS=OFF')
$arguments += if ($Vulkan) { '-DTRANSCRIBE_VULKAN=ON' } else { '-DTRANSCRIBE_VULKAN=OFF' }
if ($Gnu) { $arguments += @('-G', 'Ninja', '-DCMAKE_C_COMPILER=x86_64-w64-mingw32-clang', '-DCMAKE_CXX_COMPILER=x86_64-w64-mingw32-clang++') }
else { $arguments += @('-A', 'x64') }
cmake @arguments
if ($LASTEXITCODE -ne 0) { throw 'Native configure failed' }
cmake --build $buildRoot --config Release --parallel 6
if ($LASTEXITCODE -ne 0) { throw 'Native build failed' }
cmake --install $buildRoot --config Release --prefix $prefix
if ($LASTEXITCODE -ne 0) { throw 'Native install failed' }
$env:TRANSCRIBE_DIR = $prefix
$env:DICTADO_GNU = if ($Gnu) { '1' } else { '0' }
Write-Output "Native engine ready: $prefix"
