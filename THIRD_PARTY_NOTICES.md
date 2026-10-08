# Third-party notices

Handy source/history: MIT, copyright 2025 CJ Pais; full notice in LICENSE.
transcribe.cpp/transcribe-cpp-sys 0.2.4: MIT, copyright 2026 transcribe.cpp authors.
ggml: MIT, copyright 2023–2026 ggml authors. Exact texts are in licenses/.
The Parakeet-only native-source modifications are described in docs/provenance.md.

Rust runtime crates: hound (Apache-2.0), serde/serde_json, sha2 and their locked
dependencies (MIT/Apache-2.0 or their declared compatible licenses). GNU runtime
DLLs use LLVM libc++/libunwind (Apache-2.0 with LLVM exceptions). Build tools are
development prerequisites, not installer payload. The full resolved license list
and all texts for the actual DLL/static payload must be staged before release.

The selected future bundled model is NVIDIA Parakeet TDT 0.6B v3, converted and
quantized by handy-computer, CC BY 4.0. Attribution and exact model metadata are
in models/manifest.json. Distribution must include the CC BY 4.0 license text and
notices for all shipped C++/Rust libraries. This file must be updated with the
actual payload before any installer release. There is no installer at this milestone.
