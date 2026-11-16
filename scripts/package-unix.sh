#!/bin/sh
# Build the Linux or macOS terminal binary and pack it, with its installer and
# license notices, into artifacts/Dictado-Lite-<version>-<platform>.tar.gz.
# Runs on the system it packages; no model weights are included.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)
case "$(uname -s)-$(uname -m)" in
Linux-x86_64) platform=Linux-x64 ;;
Linux-aarch64) platform=Linux-arm64 ;;
Darwin-arm64) platform=macOS-arm64 ;;
Darwin-x86_64) platform=macOS-x64 ;;
*)
    echo "Unsupported system: $(uname -s) $(uname -m)" >&2
    exit 1
    ;;
esac
name="Dictado-Lite-$version-$platform"
stage="artifacts/$name"
# A separate target folder keeps these flags from rebuilding the everyday one.
target="target/package"
# Never tune the engine for the build machine: the package runs on other CPUs.
cmake_args="-DTRANSCRIBE_USE_SYSTEM_BLAS=OFF -DGGML_NATIVE=OFF"

rm -rf "$stage" "$stage.tar.gz"
mkdir -p "$stage"
case $platform in
Linux-*)
    # The engine becomes shared libraries plus one CPU module per instruction
    # set tier, chosen at run time, so one package is fast on new processors
    # and still runs on old ones. The binary finds them in its lib folder.
    TRANSCRIBE_CMAKE_ARGS="$cmake_args" RUSTFLAGS='-C link-arg=-Wl,-rpath,$ORIGIN/lib' \
        cargo build --locked --release --bin dictado-lite --target-dir "$target" \
        --features transcribe-cpp-sys/dynamic-backends
    # The installed lib folder holds the engine libraries and the CPU modules.
    engine=$(dirname "$(ls -t "$target"/release/build/transcribe-cpp-sys-*/out/lib/transcribe-link.json | head -n 1)")
    mkdir "$stage/lib"
    cp -P "$engine"/*.so* "$stage/lib/"
    ;;
macOS-*)
    # One static binary with the Metal backend embedded and CPU fallback.
    TRANSCRIBE_CMAKE_ARGS="$cmake_args" \
        cargo build --locked --release --bin dictado-lite --target-dir "$target"
    ;;
esac

cp "$target/release/dictado-lite" scripts/install.sh LICENSE NOTICE THIRD_PARTY_NOTICES.md "$stage/"
cp -R licenses "$stage/"
chmod 755 "$stage/dictado-lite" "$stage/install.sh"
tar -C artifacts -czf "$stage.tar.gz" "$name"
echo "$stage.tar.gz"
