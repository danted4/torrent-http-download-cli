#!/usr/bin/env bash
# Build libtorrenter.so for Android ABIs and regenerate Kotlin bindings.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required for Android targets (aarch64-linux-android, x86_64-linux-android)" >&2
  exit 1
fi

rustup target add aarch64-linux-android x86_64-linux-android

cargo build --features ffi
cargo run --features ffi-bindgen --bin uniffi-bindgen -- generate \
  --library target/debug/libtorrenter.so \
  --language kotlin \
  --out-dir android/app/src/main/java \
  --no-format

if command -v cargo-ndk >/dev/null 2>&1; then
  cargo ndk -P 30 -t arm64-v8a -t x86_64 -o android/app/src/main/jniLibs \
    build --locked --features ffi --release --lib
  exit 0
fi

if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  echo "install cargo-ndk, or set ANDROID_NDK_HOME and re-run" >&2
  exit 1
fi

HOST="$(uname -s | tr '[:upper:]' '[:lower:]')"
PREBUILT="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/${HOST}-x86_64/bin"
API=30

build_abi() {
  local rust_target="$1"
  local abi="$2"
  local clang="$3"
  export CC="$PREBUILT/$clang"
  export AR="$PREBUILT/llvm-ar"
  export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$PREBUILT/$clang"
  export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$PREBUILT/x86_64-linux-android${API}-clang"
  cargo build --features ffi --release --target "$rust_target"
  local dest="$ROOT/android/app/src/main/jniLibs/$abi"
  mkdir -p "$dest"
  cp "$ROOT/target/$rust_target/release/libtorrenter.so" "$dest/"
}

build_abi aarch64-linux-android arm64-v8a "aarch64-linux-android${API}-clang"
build_abi x86_64-linux-android x86_64 "x86_64-linux-android${API}-clang"
