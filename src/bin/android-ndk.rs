//! Host helper: UniFFI Kotlin + `libtorrenter.so` for Android ABIs.
//! Not an updater asset. Usage: `cargo run --features android-ndk --bin android-ndk`

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

const API: u32 = 30;
const ANDROID_TARGETS: [&str; 2] = ["aarch64-linux-android", "x86_64-linux-android"];

fn main() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    env::set_current_dir(&root).with_context(|| format!("cd {}", root.display()))?;

    if Command::new("rustup").arg("--version").status().is_err() {
        bail!(
            "rustup is required for Android targets (aarch64-linux-android, x86_64-linux-android)"
        );
    }

    let mut add = Command::new("rustup");
    add.args(["target", "add"]).args(ANDROID_TARGETS);
    run(&mut add)?;

    run(Command::new("cargo").args(["build", "--features", "ffi"]))?;

    let host_lib = host_cdylib(&root);
    if !host_lib.is_file() {
        bail!("missing host library {}", host_lib.display());
    }
    run(Command::new("cargo")
        .args([
            "run",
            "--features",
            "ffi-bindgen",
            "--bin",
            "uniffi-bindgen",
            "--",
            "generate",
            "--library",
        ])
        .arg(&host_lib)
        .args([
            "--language",
            "kotlin",
            "--out-dir",
            "android/app/src/main/java",
            "--no-format",
        ]))?;

    if Command::new("cargo-ndk").arg("--version").status().is_ok() {
        return run(Command::new("cargo").args([
            "ndk",
            "-P",
            "30",
            "-t",
            "arm64-v8a",
            "-t",
            "x86_64",
            "-o",
            "android/app/src/main/jniLibs",
            "build",
            "--locked",
            "--features",
            "ffi",
            "--release",
            "--lib",
        ]));
    }

    let ndk = env::var("ANDROID_NDK_HOME").unwrap_or_default();
    if ndk.is_empty() {
        bail!("install cargo-ndk, or set ANDROID_NDK_HOME and re-run");
    }
    let prebuilt = PathBuf::from(&ndk)
        .join("toolchains/llvm/prebuilt")
        .join(format!("{}-x86_64", ndk_host_os()))
        .join("bin");
    build_abi(
        &root,
        &prebuilt,
        "aarch64-linux-android",
        "arm64-v8a",
        &format!("aarch64-linux-android{API}-clang"),
    )?;
    build_abi(
        &root,
        &prebuilt,
        "x86_64-linux-android",
        "x86_64",
        &format!("x86_64-linux-android{API}-clang"),
    )
}

fn build_abi(
    root: &Path,
    prebuilt: &Path,
    rust_target: &str,
    abi: &str,
    clang: &str,
) -> Result<()> {
    let clang_path = prebuilt.join(clang);
    let ar = prebuilt.join("llvm-ar");
    let mut cmd = Command::new("cargo");
    cmd.args([
        "build",
        "--features",
        "ffi",
        "--release",
        "--target",
        rust_target,
    ])
    .env("CC", &clang_path)
    .env("AR", &ar)
    .env(
        "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER",
        prebuilt.join(format!("aarch64-linux-android{API}-clang")),
    )
    .env(
        "CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER",
        prebuilt.join(format!("x86_64-linux-android{API}-clang")),
    );
    run(&mut cmd)?;
    let dest = root.join("android/app/src/main/jniLibs").join(abi);
    fs::create_dir_all(&dest).with_context(|| format!("mkdir {}", dest.display()))?;
    let so = root
        .join("target")
        .join(rust_target)
        .join("release")
        .join("libtorrenter.so");
    fs::copy(&so, dest.join("libtorrenter.so"))
        .with_context(|| format!("copy {} -> {}", so.display(), dest.display()))?;
    Ok(())
}

fn host_cdylib(root: &Path) -> PathBuf {
    let debug = root.join("target/debug");
    if cfg!(windows) {
        debug.join("torrenter.dll")
    } else if cfg!(target_os = "macos") {
        debug.join("libtorrenter.dylib")
    } else {
        debug.join("libtorrenter.so")
    }
}

fn ndk_host_os() -> &'static str {
    if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(windows) {
        "windows"
    } else {
        "linux"
    }
}

fn run(cmd: &mut Command) -> Result<()> {
    let status = cmd
        .status()
        .with_context(|| format!("failed to spawn {:?}", cmd.get_program()))?;
    if !status.success() {
        bail!("{:?} failed with {status}", cmd.get_program());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_cdylib_sits_in_debug() {
        let path = host_cdylib(Path::new("repo"));
        let name = path.file_name().unwrap().to_string_lossy();
        assert!(name.contains("torrenter"), "{name}");
        assert_eq!(path.parent().and_then(|p| p.file_name()).unwrap(), "debug");
    }

    #[test]
    fn ndk_prebuilt_os_is_lowercase() {
        let host = ndk_host_os();
        assert!(matches!(host, "linux" | "darwin" | "windows"));
    }
}
