# Third-party notices

Handy source/history: MIT, copyright 2025 CJ Pais; full notice in LICENSE.
transcribe.cpp/transcribe-cpp-sys 0.2.4: MIT, copyright 2026 transcribe.cpp authors.
ggml: MIT, copyright 2023–2026 ggml authors. Exact texts are in licenses/.
The full native engine is built from the pinned archive; provenance is in docs/provenance.md.
miniz 3.1.1: MIT, copyright RAD Game Tools/Valve Software and Rich Geldreich/
Tenacious Software LLC; full notice in licenses/miniz-LICENSE.txt.

Rust crates include CPAL and hound (Apache-2.0), Rubato/RealFFT (MIT), earshot,
rtrb, serde/serde_json, sha2, Windows bindings and their locked dependencies.
The installer supplies the resolved inventory and original crate notices under
licenses/rust, including Unicode-3.0 where applicable. Rust standard-library
copyright/license notices are supplied under licenses/rust-std. GNU runtime DLLs
use LLVM libc++/libunwind (Apache-2.0 with LLVM exceptions); full LLVM and MinGW
runtime texts are included. Build tools themselves are not installer payload.
See licenses/SOURCES.md for exact origins and archive license-file overrides.

No model weights are bundled in the installer. The recommended downloadable model
is NVIDIA Parakeet TDT 0.6B v3, converted and quantized by handy-computer, CC BY 4.0.
Its attribution and exact metadata are in models/manifest.json; full terms are in
licenses/CC-BY-4.0.txt. Conversion/quantization change NVIDIA's original format.

Handy's MIT-licensed GGUF catalog is vendored in models/catalog.json with original
ownership and source recorded in NOTICE. Other models have independent licenses,
including model-specific terms marked "other" and noncommercial licenses. The
application links to source model details; license metadata stays in the catalog. The
application's MIT license does not relicense model weights. No Handy, NVIDIA or
handy-computer endorsement is claimed.
