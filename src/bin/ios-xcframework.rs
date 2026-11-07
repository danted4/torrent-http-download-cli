//! Host helper: UniFFI Swift + `TorrenterFFI.xcframework` for iOS device and simulator.
//! Not an updater asset. macOS only. Usage: `cargo run --features ios-xcframework --bin ios-xcframework`

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

const IOS_TARGETS: [&str; 2] = ["aarch64-apple-ios", "aarch64-apple-ios-sim"];
const DEPLOY: &str = "16.0";

fn main() -> Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    env::set_current_dir(&root).with_context(|| format!("cd {}", root.display()))?;

    if !cfg!(target_os = "macos") {
        bail!("ios-xcframework only runs on macOS (needs Xcode, rustup iOS targets)");
    }
    if Command::new("rustup").arg("--version").status().is_err() {
        bail!("rustup is required for iOS targets (aarch64-apple-ios, aarch64-apple-ios-sim)");
    }
    let mut add = Command::new("rustup");
    add.args(["target", "add"]).args(IOS_TARGETS);
    run(&mut add)?;

    run(Command::new("cargo").args(["build", "--locked", "--features", "ffi"]))?;

    let host_lib = host_cdylib(&root);
    if !host_lib.is_file() {
        bail!("missing host library {}", host_lib.display());
    }

    let generated = root.join("ios/generated");
    fs::create_dir_all(&generated).with_context(|| format!("mkdir {}", generated.display()))?;
    run(Command::new("cargo")
        .args([
            "run",
            "--locked",
            "--features",
            "ffi-bindgen",
            "--bin",
            "uniffi-bindgen",
            "--",
            "generate",
            "--library",
        ])
        .arg(&host_lib)
        .args(["--language", "swift", "--out-dir"])
        .arg(&generated)
        .args(["--no-format"]))?;

    let headers = generated.join("headers");
    fs::create_dir_all(&headers).with_context(|| format!("mkdir {}", headers.display()))?;
    fs::copy(
        generated.join("torrenterFFI.h"),
        headers.join("torrenterFFI.h"),
    )
    .context("copy torrenterFFI.h")?;
    let modulemap = fs::read_to_string(generated.join("torrenterFFI.modulemap"))
        .context("read torrenterFFI.modulemap")?;
    fs::write(headers.join("module.modulemap"), modulemap).context("write module.modulemap")?;

    require_xcode()?;

    for target in IOS_TARGETS {
        let mut cmd = Command::new("cargo");
        cmd.args([
            "build",
            "--locked",
            "--features",
            "ffi",
            "--release",
            "--lib",
            "--target",
            target,
        ])
        .env("IPHONEOS_DEPLOYMENT_TARGET", DEPLOY);
        run(&mut cmd)?;
        let archive = root
            .join("target")
            .join(target)
            .join("release")
            .join("libtorrenter.a");
        if !archive.is_file() {
            bail!("missing staticlib {}", archive.display());
        }
    }

    let xc = root.join("ios/TorrenterFFI.xcframework");
    if xc.exists() {
        fs::remove_dir_all(&xc).with_context(|| format!("rm {}", xc.display()))?;
    }
    let device_lib = root.join("target/aarch64-apple-ios/release/libtorrenter.a");
    let sim_lib = root.join("target/aarch64-apple-ios-sim/release/libtorrenter.a");
    run(Command::new("xcodebuild")
        .args(["-create-xcframework", "-library"])
        .arg(&device_lib)
        .args(["-headers"])
        .arg(&headers)
        .args(["-library"])
        .arg(&sim_lib)
        .args(["-headers"])
        .arg(&headers)
        .args(["-output"])
        .arg(&xc))?;
    Ok(())
}

fn require_xcode() -> Result<()> {
    let status = Command::new("xcodebuild")
        .arg("-version")
        .status()
        .context("spawn xcodebuild")?;
    if !status.success() {
        bail!(
            "full Xcode is required (iphoneos SDK), not Command Line Tools only. Install Xcode from the App Store, then: sudo xcode-select -s /Applications/Xcode.app/Contents/Developer"
        );
    }
    Ok(())
}

fn host_cdylib(root: &Path) -> PathBuf {
    root.join("target/debug/libtorrenter.dylib")
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
        assert_eq!(path.file_name().unwrap(), "libtorrenter.dylib");
        assert_eq!(path.parent().and_then(|p| p.file_name()).unwrap(), "debug");
    }

    #[test]
    fn ios_targets_are_apple() {
        assert!(IOS_TARGETS
            .iter()
            .all(|target| target.contains("apple-ios")));
    }
}
