# Third-party notices

Handy source/history: MIT, copyright 2025 CJ Pais; full notice in LICENSE.
transcribe.cpp/transcribe-cpp-sys 0.2.4: MIT, copyright 2026 transcribe.cpp authors.
ggml: MIT, copyright 2023–2026 ggml authors. Exact texts are in licenses/.
The Parakeet-only native-source modifications are described in docs/provenance.md.

Rust crates include CPAL and hound (Apache-2.0), Rubato/RealFFT (MIT), earshot,
rtrb, serde/serde_json, sha2, Windows bindings and their locked dependencies.
The installer supplies the resolved inventory and original crate notices under
licenses/rust, including Unicode-3.0 where applicable. Rust standard-library
copyright/license notices are supplied under licenses/rust-std. GNU runtime DLLs
use LLVM libc++/libunwind (Apache-2.0 with LLVM exceptions); full LLVM and MinGW
runtime texts are included. Build tools themselves are not installer payload.
See licenses/SOURCES.md for exact origins and archive license-file overrides.

The bundled model is NVIDIA Parakeet TDT 0.6B v3, converted and
quantized by handy-computer, CC BY 4.0. Attribution and exact model metadata are
in models/manifest.json. Its full CC BY 4.0 terms are in licenses/CC-BY-4.0.txt.
The format conversion and Q8_0 quantization differ from NVIDIA's original model;
this project does not claim NVIDIA or handy-computer endorsement. Distribution
includes one pinned GGUF, native MIT engine/ggml notices and the above libraries.
