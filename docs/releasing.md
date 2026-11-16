# Publish a release

Merging source and publishing a release are separate steps, but one workflow
(**Build**) does both. Every pull request runs it on Linux (format, lint,
tests) and Windows (the same plus a release build), with cached tools, native
engine and cargo output. The manual release run repeats only the Windows job,
builds the installer and drafts the release. macOS is checked only when asked
(`check_macos`), because GitHub bills it at ten times the Linux minutes.
Release source must be tagged and already merged into `main`; start the
workflow from `main` so caches can be reused between versions.

1. Align `Cargo.toml`, `Cargo.lock` and `assets/app.rc` with the intended version.
   Update `docs/release-notes.md` and the README's installer filename.
2. Review and merge the pull request once **Build** is green. Code changes
   should also receive relevant local checks; documentation changes need no
   installer build.
3. Tag the merged commit as `dictado-v<version>` and push that tag to `origin`.
   This does not launch a workflow. Then run:

   ```powershell
   gh workflow run native.yml --ref main -f release_tag=dictado-v<version>
   ```

4. Wait for **Windows x64** and **Draft release** to complete. The run checks
   the tagged source on Windows, packages it without weights and creates a
   draft release. It does not download models. Build tools, the native engine
   and cargo output are cached on `main`; the first manual build is slower.
   Leaving `release_tag` empty checks the source without creating a release.
5. Inspect the draft's version, English notes and asset. It must contain exactly
   one `Dictado-Lite-<version>-Setup.exe`, with no GGUF model or checksum companion.
6. Publish the verified draft and mark it latest. Confirm that `/releases/latest`
   resolves to the new version and that the installer asset is uploaded.

Do not overwrite a published tag. If a workflow fails before draft creation,
fix the problem or rerun the failed job; do not publish an incomplete package.
Only the Windows installer is published; Linux and macOS users build the
terminal binary from source. The native executable is currently unsigned.
The release notes and README state that limitation.
