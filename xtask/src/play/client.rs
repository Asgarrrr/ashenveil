//! Builds the web client into its git-ignored directories: the WASM package
//! of `ash-wasm` into `web/pkg/`, then the TypeScript into `web/build/`.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::{run_step, target_dir};
use crate::verify::toolchain::cargo;

pub fn build(root: &Path) -> Result<(), String> {
    let wasm = build_wasm(root)?;
    let lock = std::fs::read_to_string(root.join("Cargo.lock"))
        .map_err(|error| format!("Cargo.lock: {error}"))?;
    let locked = locked_version(&lock, "wasm-bindgen")
        .ok_or("Cargo.lock pins no `wasm-bindgen`: is `ash-wasm` in the workspace?")?;
    let cli = Command::new("wasm-bindgen")
        .arg("--version")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default();
    require_pinned_cli(locked, &cli)?;
    let web = root.join("web");
    run_step(
        Command::new("wasm-bindgen")
            .args(["--target", "web", "--out-dir"])
            .arg(web.join("pkg"))
            .arg(wasm),
    )?;
    if !web.join("node_modules/typescript").is_dir() {
        run_step(
            Command::new("npm")
                .args(["ci", "--no-audit", "--no-fund"])
                .current_dir(&web),
        )?;
    }
    run_step(
        Command::new("node")
            .arg(web.join("node_modules/typescript/bin/tsc"))
            .arg("--project")
            .arg(&web),
    )
}

/// Builds `ash-wasm` for the browser; returns the path of its `.wasm`.
fn build_wasm(root: &Path) -> Result<PathBuf, String> {
    const TARGET: &str = "wasm32-unknown-unknown";
    run_step(
        Command::new(cargo())
            .args([
                "build",
                "--release",
                "--package",
                "ash-wasm",
                "--target",
                TARGET,
            ])
            .current_dir(root),
    )?;
    Ok(target_dir(root)?.join(TARGET).join("release/ash_wasm.wasm"))
}

/// The version of `package` that `lock`, a `Cargo.lock`, pins.
fn locked_version<'a>(lock: &'a str, package: &str) -> Option<&'a str> {
    let name = format!("name = \"{package}\"");
    let mut lines = lock.lines();
    lines.find(|line| *line == name)?;
    lines
        .next()?
        .strip_prefix("version = \"")?
        .strip_suffix('"')
}

/// The wasm-bindgen CLI writes the JS glue for one version of the crate
/// only: any other version breaks the package at load time.
fn require_pinned_cli(locked: &str, cli_version: &str) -> Result<(), String> {
    match cli_version
        .split_whitespace()
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["wasm-bindgen", version] if *version == locked => Ok(()),
        _ => Err(format!(
            "the wasm-bindgen CLI must be {locked}, as Cargo.lock pins it; found `{}`. \
             Install it with `cargo install --locked wasm-bindgen-cli --version {locked}`.",
            cli_version.trim()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCK: &str = "[[package]]\nname = \"wasm-bindgen-macro\"\nversion = \"0.2.131\"\n\n\
                        [[package]]\nname = \"wasm-bindgen\"\nversion = \"0.2.130\"\n";

    #[test]
    fn locked_version_is_the_one_of_the_named_package() {
        assert_eq!(locked_version(LOCK, "wasm-bindgen"), Some("0.2.130"));
        assert_eq!(locked_version(LOCK, "wasm-bindgen-shared"), None);
    }

    #[test]
    fn cli_of_the_locked_version_is_accepted() {
        require_pinned_cli("0.2.130", "wasm-bindgen 0.2.130\n").unwrap();
    }

    #[test]
    fn cli_of_another_version_is_refused_with_its_install_command() {
        let error = require_pinned_cli("0.2.130", "wasm-bindgen 0.2.131\n").unwrap_err();
        assert!(
            error.contains("cargo install --locked wasm-bindgen-cli --version 0.2.130"),
            "{error}"
        );
    }

    #[test]
    fn missing_cli_is_refused() {
        assert!(require_pinned_cli("0.2.130", "").is_err());
    }
}
