# Installed native app · 2026-10-08

Windows 11 x64, Ryzen 5 5500X3D / Radeon RX580, same pinned Parakeet Q8_0.
Actual GNU release payload; private recordings/transcripts stay local. This is
physical acceptance on the development PC with an isolated deployment, not a
claim that a fresh Windows VM was tested.

## Installer and dependencies

- Native setup stub 693,760 bytes; application 2,007,040 bytes; one GGUF
  739,508,576 bytes with exact published SHA-256.
- Full offline setup 794,975,092 bytes (about 758 MiB), including runtime DLLs,
  model and licenses. Final whole-file digest is provided beside the artifact.
- `--test --quiet` installed the complete payload in 2.08 s with PATH restricted
  to Windows directories. Upgrade to the final build took 2.74 s. Tiny-stub
  uninstall completed in 0.37 s and left only the deliberately unowned synthetic
  file; that test-only file was then removed explicitly.
- Normal per-user install created the Start Menu shortcut and own HKCU uninstall
  entry. No startup entry is enabled by setup. No stale stage directory remained.
- PE import review: all non-system libraries are included; Vulkan loader/driver
  belongs to the OS/GPU installation. No Handy/Node/Python/WebView2/CUDA library
  is referenced. With restricted PATH the installed app used its bundled model
  and Vulkan successfully. Removing only the QA copy's Vulkan DLL caused CPU
  fallback and the same exact English baseline. Restored that DLL before cleanup.
- Safety units reject traversal/case duplicates/device paths/marker collisions,
  corrupted bytes and cancellation, and preserve unexpected files. GUI setup
  controls were observed in the real packaged executable; cancellation before
  installation was exercised. A clean VM, signing and power-loss recovery are
  outside this verification. The installer is unsigned.

## Recognition, insertion and lifetime

The normal installed tray app (no model argument, no inspection shell) inserted
the private 48.168 s WAV into an empty Notepad tab. Its saved text is exactly the
574-character accepted baseline. Cold load 626 ms, inference 3524 ms, trigger to
post-injection clipboard read 4213 ms. This is a WAV-fixture trigger and receipt
proxy, not a live key-release/actual editor-insertion timestamp. Actual insertion
was separately confirmed by saving the editor output. Live input is deferred by
the user. Automatic browser policy rejected a local-file test; browser insertion
and independent DOM timing are not claimed.

72 resource samples span 3.88–47.52 s, starting during cold inference. Working-set
peak 379.44 MiB, private peak 1227.62 MiB, GPU dedicated peak 712.91 MiB. At 8.33 s
after inference: 353.03 MiB resident / 1029.38 MiB private / 712.52 MiB dedicated
GPU. At 47.52 s after model unload: 72.90 MiB resident / 41.79 MiB private /
5.73 MiB dedicated and 9.34 MiB shared GPU. CPU stays at 4.65625 s from completion
through idle; final 9.19 s delta is zero. Exit IPC was delivered and the process
ended. No deliberate working-set trimming was used.

Compared with the accepted CLI core's unloaded 46/31.2 MiB, the complete native
tray adds about 27 MiB resident / 11 MiB private in this run. The earlier UI
checkpoint was 59.2/34.7 MiB; these are single-run endpoints with differing
clipboard/UI/allocator histories, so residency is reported rather than described
as a memory improvement. Loaded private/GPU residency remains close to the core
baseline (~1028/712.5 MiB). The compact native UI/recovery and packaged app justify
their small absolute overhead; model lifetime avoids persistent loaded memory.

An installed full-WAV CLI repeat produced load 629/0/0 ms, inference
2833/2924/2947 ms and cleanup 47/27/61 µs, all three transcripts exact. This is
roughly 4% above the earlier 2815 ms warm median in an uncontrolled sequential
comparison. The shipped native engine bytes and numerical path are unchanged;
no speed improvement is claimed. Cold whole-app inference remains comparable
with the accepted core cold 3420 ms. More repetitions under controlled GPU clocks
would be needed to attribute small timing differences to code.
