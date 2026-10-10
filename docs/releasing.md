# Publish a Windows release

Merging source and publishing a release are separate steps. Actions runs only
when requested manually. Release source must be tagged and already merged into
`main`; start the workflow from `main` so caches can be reused between versions.

1. Align `Cargo.toml`, `Cargo.lock` and `assets/app.rc` with the intended version.
   Update `docs/release-notes.md` and the README's installer filename.
2. Review and merge the pull request. Code changes should receive relevant local
   checks; documentation changes need no installer build.
3. Tag the merged commit as `dictado-v<version>` and push that tag to `origin`.
   This does not launch a workflow. Then run:

   ```powershell
   gh workflow run native.yml --ref main -f release_tag=dictado-v<version>
   ```

4. Wait for **Build installer** to complete. The workflow compiles tagged source,
   packages it without weights and creates a draft release. It does not download
   models or run a compatibility/test suite. Build tools and the native engine
   are cached on `main`; the first manual build is slower. Leaving `release_tag`
   empty generates an artifact without creating a release.
5. Inspect the draft's version, English notes and asset. It must contain exactly
   one `Dictado-Lite-<version>-Setup.exe`, with no GGUF model or checksum companion.
6. Publish the verified draft and mark it latest. Confirm that `/releases/latest`
   resolves to the new version and that the installer asset is uploaded.

Do not overwrite a published tag. If a workflow fails before draft creation,
fix the problem or rerun the failed job; do not publish an incomplete package.
The native executable is currently unsigned. The release notes and README
state that limitation.
