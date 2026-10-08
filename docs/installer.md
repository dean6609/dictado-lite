# Offline Windows installer

`dictado-setup.exe` is a native Rust/Win32 stub. `scripts/build-installer.ps1`
appends the application, required native DLLs, one pinned GGUF model and license
texts, followed by a bounded JSON index and footer. Neither installer nor app
downloads a model or invokes Node/Python/Handy. The setup UI offers Install and
Cancel and reports copy/hash progress from a worker. Startup remains optional/off.

Install for the current user in `%LOCALAPPDATA%/Programs/DictadoLite`, create a
Start Menu shortcut and a unique HKCU uninstall entry. The tiny uninstaller
relocates itself to a unique TEMP file so Windows can release its installed image;
that inert TEMP copy may remain until ordinary temporary-file cleanup. Settings
under `%LOCALAPPDATA%/DictadoLite` are preserved, including on uninstall. No Handy
or MicFilter directories, entries or settings are involved.

Before commit, the installer checks every staged file's SHA-256 and the compiled
model size/hash pin. Paths are relative, bounded and reject traversal, device
names, case duplicates and reparse ancestors. An existing nonempty directory
without this product's marker is refused. Upgrade moves only the previous
manifest's files into a temporary backup; a failed publication rolls those moves
back. Uninstall deletes only manifest-listed files and empty directories,
preserving unexpected user files. Graceful IPC closes this app; no forced process
termination or recursive deletion is used. Cancellation stops extraction before
commit. This is not a security boundary against a hostile process in the same
user account, and a power loss during the brief commit can require reinstalling.

The executable is unsigned. The adjacent whole-file SHA-256 checks download
integrity, not publisher authentication. Distribution remains in the user's own
private GitHub repository. Release files contain no recordings/transcripts.

GNU packaging includes LLVM libc++/libunwind and MinGW runtime notices; native
MIT engine/ggml notices, Handy MIT provenance, model CC BY 4.0/attribution, resolved
Rust crate texts and Rust standard-library notices are included. Three crates
omit license files from their archives: pinned upstream MIT texts for dasp_sample
and earshot are vendored; realfft's declared MIT terms and explicit upstream
attribution/provenance are recorded without inventing a copyright year. The
packager refuses a missing dependency license instead of shipping an incomplete
notice set. Included Rust standard-library license texts accompany its own
copyright inventory; they do not change this application's MIT license.

Build after the native release checks:

```powershell
./scripts/build-installer.ps1 -Model '<pinned model.gguf>'
```

Developer-only `--quiet --test` installs into the separate fixed sibling
`DictadoLite-QA` without registry or shortcuts. `--uninstall --test --quiet` uses
its own tiny helper. This is the same extraction/hash/upgrade/delete path as the
normal installer. Regular CI compiles both binaries and runs payload safety tests;
physical packaging/insertion/resource evidence is recorded separately.
