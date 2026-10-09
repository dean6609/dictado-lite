# Build and contribute

Clone `https://github.com/dean6609/dictado-lite.git`. Use Windows x64 and Rust
1.99.0 with rustfmt/clippy. Native inference needs a C++ compiler. GitHub CI uses
Visual Studio on Windows. A local GNU build uses the pinned LLVM-MinGW
compiler downloaded by the bootstrap script; no compiler from another project
is required. Both build the same Parakeet-only native source patch.
Install [Git](https://git-scm.com/downloads/win) and [Rustup](https://rustup.rs/)
first. A fresh clone needs the GNU Rust toolchain as well as the C++ compiler:

```powershell
git clone https://github.com/dean6609/dictado-lite.git
cd dictado-lite
rustup toolchain install 1.99.0-x86_64-pc-windows-gnu --profile minimal --component rustfmt --component clippy
rustup override set 1.99.0-x86_64-pc-windows-gnu
./scripts/build-native.ps1 -Vulkan -Gnu
./scripts/check.ps1
```

Use these two commands in the same shell. If using Rust MSVC with Visual Studio,
omit `-Gnu`; omit `-Vulkan` for a CPU-only native build. The scripts download tools
and source by SHA-256, compile C++, run format/clippy/tests/release and stage DLLs.
The first build downloads the pinned CMake/Ninja/LLVM-MinGW/Vulkan development
tools and native source. They stay in ignored `.tools/`; subsequent builds reuse
them. MSVC development/CI needs Visual Studio C++ tools and the MSVC Rust
toolchain; consumer installer packaging currently requires GNU.

The regular checks need no model. To run the app or package its installer, obtain
the single pinned model in the same PowerShell session:

```powershell
$pin = Get-Content ./models/manifest.json -Raw | ConvertFrom-Json
New-Item -ItemType Directory -Force ./.tools/models | Out-Null
$model = Join-Path $PWD ('.tools/models/' + $pin.filename)
Invoke-WebRequest "$($pin.conversion_repository)/resolve/$($pin.revision)/$($pin.filename)" -OutFile $model
if ((Get-Item $model).Length -ne $pin.bytes -or (Get-FileHash $model).Hash -ne $pin.sha256) {
    throw 'Pinned model integrity check failed'
}
./target/release/dictado-lite.exe --model $model
```

An existing copy with the same size/hash can be reused instead of downloading.
The model is about 740 MB and stays outside Git. Development builds use an
explicit model path; the installed app uses its bundled copy:

```powershell
./target/release/dictado-lite.exe --model '<model.gguf>'
```

Hold Ctrl+Alt+Space to speak; release to recognize and paste. Escape cancels an
active session. The tray controls pause, microphone, shortcut, cleanup and optional
Windows startup. The native overlay and offline installer are implemented.

After checks, build the offline Windows x64 installer with the pinned weights:

```powershell
./scripts/build-installer.ps1 -Model '<pinned model.gguf>'
```

The output is `artifacts/Dictado-Lite-0.1.0-Setup.exe` plus SHA-256. Both native
application and setup binaries are built by check.ps1. Keep payloads outside Git.
Read [installer ownership and QA](docs/installer.md) before changing packaging.
No external installer compiler or administrator installation is needed.

```powershell
./target/release/dictado-lite.exe '<model.gguf>' '<mono-16k.wav>' --verify-model --repeat=3
```

Output is JSON with text and file-inference timing. In a GUI-subsystem release,
use `Start-Process -Wait` with `-RedirectStandardOutput` and
`-RedirectStandardError` to capture it reliably in PowerShell; the tested
measure-core.ps1 launcher does this. Store private results only in
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

`--capture-probe` captures five seconds and prints anonymous rate/RMS/voice-score
statistics without saving PCM. `--dictate-wav '<mono-16k.wav>' --model '<model.gguf>'`
exercises actual paste into the currently focused test editor. Use only a dedicated
test document. Optional `--metrics-file '<local.json>'` saves anonymous timings;
the clipboard-read receipt is a proxy, so verify the editor's actual saved text too.
`--cancel`, `--exit` and `--menu` address the running instance. Normal use writes
settings only, never recordings or transcript history.

Local desktop-only clipboard regression (temporarily replaces and restores the
clipboard; do not run while copying other content):
`cargo test --locked --lib receipt_restores -- --ignored --test-threads=1`.

For a real live-input appearance capture use `--inspect-overlay --capture-seconds=60`
with an explicit model, then cancel with Escape. This diagnostic shell mode makes
the same pill eligible for window-capture tools and adds a temporary taskbar entry.
Normal use has no main/taskbar window. `--metrics-file '<local.json>'` additionally
writes a `.ui.json` sidecar with anonymous DPI/frame/focus/preference measurements.
`--list-microphones` lists current inputs; `--capture-probe --microphone '<name>'`
selects an input for the explicit five-second diagnostic. These flags are development
fixtures, never required for normal installation or use.

`--control-report` prints anonymous IPC status with an explicit `--cancel`, `--exit`
or `--menu`. Control-only invocations do not start a new instance when none exists.
Optional `DICTADO_UI_TRACE='<local.jsonl>'` records local UI lifecycle diagnostics;
leave it unset in normal use. It does not record microphone PCM or recognized text.

`assets/icon.svg` is the owned mark; `scripts/generate-icon.ps1` regenerates its
Windows ICO using build-only GDI+. `build.rs` embeds the icon, DPI/asInvoker manifest
and version resource with LLVM windres (GNU) or the Windows SDK compiler (MSVC).

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

The repository is public. Main requires the current `Windows checks` and
`Vulkan release build` checks through branch protection. Review changes and
documentation before merging and report the review actually performed. The
initial private-repository setup received API 403 for
protection; that historical limitation does not govern the public repository.

Add `--cleanup` to the explicit WAV CLI to measure the same conservative cleanup
used by the tray. Reports include cleanup_us; raw CLI recognition is the default.
See [cleanup rules](docs/decisions/text-cleanup.md) and their protected-content tests.
