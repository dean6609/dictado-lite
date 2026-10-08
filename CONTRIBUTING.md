# Build and contribute

Clone `https://github.com/dean6609/dictado-lite.git`. Use Windows x64 and Rust
1.99.0 with rustfmt/clippy. At this bootstrap stage no C++ compiler is needed.
The local implementation environment uses an isolated GNU Rust toolchain in
`.tools/`; GitHub CI uses MSVC. Both compile the same native library.

```powershell
./scripts/check.ps1
```

This command checks formatting, warnings, tests and release compilation. It does
not run the inherited Tauri project. The current native foundation is a library,
not an installable dictation app. C++/Vulkan build prerequisites will be added
with the inference milestone and tested before documentation claims support.

The only model is pinned in [models/manifest.json](models/manifest.json). Weights
and private WAVs stay outside Git. No model is required for bootstrap checks.

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
