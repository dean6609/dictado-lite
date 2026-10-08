# Distribution notices

- CC BY 4.0: https://creativecommons.org/licenses/by/4.0/legalcode.txt
- LLVM runtime and MinGW runtime texts: copied from the pinned LLVM-MinGW
  20260922 ucrt archive used for this build; its manifest/hash is in scripts/tool-sources.json.
- Rust standard library: copyright inventory and license texts shipped with
  the isolated Rust 1.99.0 GNU toolchain (`share/doc/rust`).
- dasp_sample 0.11.0 MIT: RustAudio/dasp commit
  97c3bb9b2363c0b46ac1633858bf1054fd02a980, LICENSE-MIT.
- earshot 1.2.2 MIT: pykeio/earshot commit
  c400f15bd232dda30a297eec90235bd09066cab7, LICENSE-MIT.
- realfft 3.5.0: HEnquist/realfft commit
  d0d4eee0525fd27c96c8a046d6d107acd5ed84a6; Cargo.toml/README declare MIT and the
  author. No separate upstream LICENSE file exists. See overrides/realfft-3.5.0.
- Remaining resolved Rust crate texts: copied from the immutable crates.io
  archives selected by Cargo.lock during packaging, accompanied by generated
  RUST-DEPENDENCIES.txt in the installer. Build-only dependencies are also listed.
- transcribe.cpp and ggml: exact native MIT notices retained in this directory.
- Model: exact pin, conversion attribution and source links in models/manifest.json.

These notices retain third-party ownership; no third-party authorship is claimed.
