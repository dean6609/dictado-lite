# Working on Dictado Lite

Read CONTRIBUTING.md and docs/architecture.md first. This independent MIT project
is being extracted from Handy; upstream contribution policies do not apply here.

Commands (Windows PowerShell): `./scripts/check.ps1` runs format, clippy, tests and
release compilation. `./scripts/dev-env.ps1` selects the isolated local toolchain
when present; otherwise use Rust 1.99.0 from PATH. Run `./scripts/build-native.ps1`
in the same shell first (local GNU adds `-Gnu`, GPU adds `-Vulkan`). See CONTRIBUTING.

Source map: `native/engine/` owns the C ABI, worker and model lifetime;
`native/session.rs` rejects invalidated results; `native/audio.rs` reads regression
WAVs; `native/main.rs` is the file CLI. `scripts/` builds the native engine;
`models/manifest.json` pins weights; `docs/` describes contracts and verification.
Upstream audio/resampling/insertion references remain available in Git history.

Keep audio callbacks free of blocking/file work. Cancellation must invalidate
results before insertion. Clipboard changes must preserve previous contents and
respect target focus. Cleanup must preserve meaning and protected tokens.

Work from main in small branches, run relevant checks, review the diff and current
HEAD CI, then open a PR using the template. Only origin belongs to this project;
never push or open PRs on upstream. Keep voice, transcripts, models and local
measurements in ignored local/ or artifacts/. Never modify Handy settings.

Update README and affected docs with each PR. Record physical Windows checks and
resource measurements separately from CI; compilation alone proves neither.
