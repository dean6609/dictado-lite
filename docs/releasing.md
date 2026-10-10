# Publish a Windows release

Merging source and publishing a release are separate steps. Releases are built
from a version tag on a commit already merged into `main`.

1. Align `Cargo.toml`, `Cargo.lock` and `assets/app.rc` with the intended version.
   Update `docs/release-notes.md` and the README's installer filename.
2. Merge the pull request after the current commit passes its required checks.
3. Tag the merged commit as `dictado-v<version>` and push that tag to `origin`.
   The lightweight package workflow requires the tag to match Cargo's version.
4. Wait for **Windows checks** to pass. The workflow builds and checks the app,
   packages it without weights, runs install/update/removal smoke checks and
   creates a draft release.
5. Inspect the draft's version, English notes and asset. It must contain exactly
   one `Dictado-Lite-<version>-Setup.exe`, with no GGUF model or checksum companion.
6. Publish the verified draft and mark it latest. Confirm that `/releases/latest`
   resolves to the new version and that the installer asset is uploaded.

Do not overwrite a published tag. If a workflow fails before draft creation,
fix the problem or rerun the failed job; do not publish an unverified package.
The native executable is currently unsigned. The release notes and README
state that limitation.
