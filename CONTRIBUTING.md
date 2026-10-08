# Build and contribute

Clone `https://github.com/dean6609/dictado-lite.git`. Use Windows x64 and Rust
1.99.0 with rustfmt/clippy. Native inference needs a C++ compiler. GitHub CI uses
Visual Studio on Windows. A local GNU build uses the pinned LLVM-MinGW
compiler downloaded by the bootstrap script; no compiler from another project
is required. Both build the same Parakeet-only native source patch.

```powershell
./scripts/build-native.ps1 -Vulkan -Gnu
./scripts/check.ps1
```

Use these two commands in the same shell. If using Rust MSVC with Visual Studio,
omit `-Gnu`; omit `-Vulkan` for a CPU-only native build. The scripts download tools
and source by SHA-256, compile C++, run format/clippy/tests/release and stage DLLs.
The current executable is a file regression tool; the tray app is still pending.

```powershell
./target/release/dictado-lite.exe '<model.gguf>' '<mono-16k.wav>' --verify-model --repeat=3
```

Output is JSON with text and file-inference timing. Store private results only in
ignored `local/`. This is not a release-to-paste measurement. Native cancellation,
session recovery and model reload smoke (synthetic silence, no private voice):

```powershell
$env:DICTADO_TEST_MODEL = '<model.gguf>'
./scripts/stage-runtime.ps1 -Directory target/release/deps
cargo test --locked --release --test native_smoke -- --ignored
```

Set `DICTADO_TEST_CPU=1` to exercise CPU explicitly. `scripts/smoke-native.ps1`
obtains the pinned model if no path is supplied, verifies size/hash and runs this
smoke. Regular cargo tests mark it ignored; CI's CPU job explicitly downloads the
weights and runs it separately. Ignoring is not a pass.

For a local model-lifetime resource trace (forty seconds idle after inference):
`./scripts/measure-core.ps1 -Model '<model.gguf>' -Wav '<mono-16k.wav>'`.
The script keeps text/logs/metrics in ignored local/; publish only reviewed,
anonymous numeric results. See [core baseline](benchmarks/core-2026-10-08.md).

The only model is pinned in [models/manifest.json](models/manifest.json). Weights
and private WAVs stay outside Git. No model is required for the regular checks.

Read [AGENTS.md](AGENTS.md), [architecture](docs/architecture.md) and
[verification](docs/verification.md) for the source map and behavior contracts.
Work on a branch from current main, run the affected checks, inspect the diff,
and open a PR using the project template. Describe the trigger, resulting
behavior, validation and limitations. AI assistance should be disclosed; it is
not a substitute for a human approval required by repository policy.

Before merging, verify CI for the current HEAD and resolve review findings.
Review README and affected documentation against the implementation and record
that review in the PR, even when no edits are needed. Prefer squash merge and
delete the merged branch. Never publish this project's branches to Handy.

Branch protection is unavailable for this private repository on the current
GitHub plan (API returned 403). Keep it private and apply the same check/review
criteria manually; do not use that limitation to merge failing or stale checks.
