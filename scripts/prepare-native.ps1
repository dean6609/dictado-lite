param([Parameter(Mandatory)][string]$SourceRoot)
$ErrorActionPreference = 'Stop'
# Keep the full upstream architecture registry for Handy's GGUF catalog.
# LLVM-MinGW needs the standard functional header for Vulkan declarations.
$typesPath = Join-Path $SourceRoot 'ggml/src/ggml-vulkan/ggml-vulkan-types.h'
$types = Get-Content -LiteralPath $typesPath -Raw
if (!$types.Contains('#include <functional>')) {
    Set-Content -LiteralPath $typesPath -Value ("#include <functional>`n" + $types) -Encoding utf8
}
