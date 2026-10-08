param([Parameter(Mandatory)][string]$SourceRoot)
$ErrorActionPreference = 'Stop'
# Deterministic, narrowly scoped changes to a hash-verified upstream archive.
# Keep public ABI headers/initializers but compile only Parakeet inference.
$cmakePath = Join-Path $SourceRoot 'src/CMakeLists.txt'
$cmake = Get-Content -LiteralPath $cmakePath -Raw
$selected = @('transcribe.cpp','transcribe-loader.cpp','transcribe-arch.cpp','transcribe-meta.cpp','transcribe-kaldi-fbank.cpp','transcribe-mel.cpp','transcribe-model.cpp','transcribe-tokenizer.cpp','transcribe-unicode.cpp','transcribe-unicode-data.cpp','transcribe-debug.cpp','transcribe-env.cpp','transcribe-flash-policy.cpp','transcribe-backend.cpp','transcribe-load-common.cpp','transcribe-weights-util.cpp','transcribe-batch-util.cpp','conformer/conformer.cpp','arch/parakeet/model.cpp','arch/parakeet/capabilities.cpp','arch/parakeet/weights.cpp','arch/parakeet/encoder.cpp','arch/parakeet/decoder.cpp')
$sourceList = "add_library(transcribe`n    " + ($selected -join "`n    ") + "`n)"
if ([regex]::Matches($cmake, 'add_library\(transcribe\b').Count -ne 1) { throw 'Unexpected native source list' }
$cmake = [regex]::Replace($cmake, '(?s)add_library\(transcribe\b.*?\n\)', $sourceList, 1)
Set-Content -LiteralPath $cmakePath -Value $cmake -Encoding utf8
@'
// Dictado Lite modification: only Parakeet can be selected.
#include "transcribe-arch.h"
#include <cstring>
namespace transcribe {
namespace parakeet { extern const Arch arch; }
const Arch * find_arch(const char * name) {
    return name != nullptr && std::strcmp(name, "parakeet") == 0 ? &parakeet::arch : nullptr;
}
}
'@ | Set-Content -LiteralPath (Join-Path $SourceRoot 'src/transcribe-arch.cpp') -Encoding utf8
$dispatchPath = Join-Path $SourceRoot 'src/transcribe.cpp'
$dispatch = Get-Content -LiteralPath $dispatchPath -Raw
$dispatch = $dispatch.Replace('#include "arch/whisper/bin_load.h"', '// Dictado Lite: legacy Whisper loader excluded.')
$dispatch = $dispatch.Replace('return transcribe::whisper::load_from_bin(path, params, out_model);', 'return TRANSCRIBE_ERR_UNSUPPORTED_ARCH; // Dictado Lite: GGUF Parakeet only.')
if ($dispatch.Contains('whisper::load_from_bin')) { throw 'Legacy loader still referenced' }
Set-Content -LiteralPath $dispatchPath -Value $dispatch -Encoding utf8
$typesPath = Join-Path $SourceRoot 'ggml/src/ggml-vulkan/ggml-vulkan-types.h'
$types = Get-Content -LiteralPath $typesPath -Raw
if (!$types.Contains('#include <functional>')) { Set-Content -LiteralPath $typesPath -Value ("#include <functional>`n" + $types) -Encoding utf8 }

# Offline TDT v3 has no embedded diarizer. Reject bundles instead of linking
# their optional Sortformer model, multi-speaker orchestration and scratch.
$modelPath = Join-Path $SourceRoot 'src/arch/parakeet/model.cpp'
$model = Get-Content -LiteralPath $modelPath -Raw
$model = [regex]::Replace($model, '(?ms)^    // Multitalker bundle: claim.*?(?=^    gguf_free\(gguf_data\);)', @'
    // Dictado Lite restriction: embedded diarizers are outside the product.
    const int64_t embedded = gguf_find_key(gguf_data, "stt.parakeet.diarizer.embedded");
    if (embedded >= 0 && gguf_get_val_bool(gguf_data, embedded)) {
        gguf_free(gguf_data);
        return TRANSCRIBE_ERR_UNSUPPORTED_ARCH;
    }

'@)
$model = [regex]::Replace($model, '(?ms)^    // Multitalker bundle post-load:.*?(?=^    m->t_load_us)', '')
$model = $model.Replace('return run_multitalker(pc, pm, pcm, n_samples, params);', 'return TRANSCRIBE_ERR_UNSUPPORTED_ARCH; // Dictado Lite: no multi-speaker path.')
if ($model -match 'sortformer::(init_embedded|fuse_embedded)|return run_multitalker') { throw 'Unpruned diarizer dependency' }
Set-Content -LiteralPath $modelPath -Value $model -Encoding utf8
$headerPath = Join-Path $SourceRoot 'src/arch/parakeet/parakeet.h'
$header = Get-Content -LiteralPath $headerPath -Raw
$header = $header.Replace('transcribe::sortformer::DiarStreamScratch diar_scratch;', '// Dictado Lite: optional diarizer scratch excluded.')
Set-Content -LiteralPath $headerPath -Value $header -Encoding utf8
