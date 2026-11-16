//! The `verify` steps that run the Rust toolchain: fmt, clippy, tests and
//! cargo-deny, plus the `cargo metadata` call shared by the manifest checks.
//! The lint guardrails live in the workspace manifests and the crate roots;
//! the tests below prove that the clippy step enforces them.

use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;

pub struct Step {
    pub name: &'static str,
    args: &'static [&'static str],
    /// How to install the cargo plugin behind this step, when rustup does not.
    install: Option<&'static str>,
}

pub const STEPS: &[Step] = &[
    Step {
        name: "fmt",
        args: &["fmt", "--all", "--check"],
        install: None,
    },
    Step {
        name: "clippy",
        args: &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        install: None,
    },
    Step {
        name: "test",
        args: &["test", "--workspace"],
        install: None,
    },
    Step {
        name: "cargo-deny",
        args: &["deny", "check"],
        install: Some("cargo install --locked cargo-deny@0.20.2"),
    },
];

impl Step {
    /// Runs the step in `root`, output streamed. A missing plugin fails the
    /// step: skipping it would silently drop a guardrail.
    pub fn run(&self, root: &Path) -> bool {
        if let Some(install) = self.install
            && let Err(message) = require_subcommand(self.args[0], install)
        {
            println!("verify: cargo {}", self.args.join(" "));
            eprintln!("verify: {message}");
            return false;
        }
        run_cargo(root, self.args)
    }

    #[cfg(test)]
    fn command(&self, root: &Path) -> Command {
        let mut command = Command::new(cargo());
        command.args(self.args).current_dir(root);
        command
    }
}

/// Runs cargo with `args` in `root`, output streamed; true on success.
pub fn run_cargo(root: &Path, args: &[&str]) -> bool {
    println!("verify: cargo {}", args.join(" "));
    match Command::new(cargo()).args(args).current_dir(root).status() {
        Ok(status) => status.success(),
        Err(error) => {
            eprintln!("verify: cannot run cargo: {error}");
            false
        }
    }
}

/// The cargo that runs xtask, so every step uses the pinned toolchain.
pub fn cargo() -> OsString {
    std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into())
}

/// Workspace members with their declared dependencies. `--no-deps` skips
/// resolving the graph: no network, no lockfile change.
pub fn workspace_metadata(root: &Path) -> io::Result<Value> {
    let output = Command::new(cargo())
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"))
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    serde_json::from_slice(&output.stdout).map_err(io::Error::other)
}

pub fn unexpected_metadata() -> io::Error {
    io::Error::other("unexpected `cargo metadata` output")
}

fn require_subcommand(subcommand: &str, install: &str) -> Result<(), String> {
    let found = Command::new(cargo())
        .args([subcommand, "--version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if found {
        Ok(())
    } else {
        Err(format!(
            "`cargo {subcommand}` is not installed. Install it with `{install}`."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_repo::{TestRepo, scratch_workspace};
    use super::*;

    fn step(name: &str) -> &'static Step {
        STEPS.iter().find(|step| step.name == name).unwrap()
    }

    /// Runs the real clippy step on `repo`; returns success and stderr.
    fn clippy(repo: &TestRepo) -> (bool, String) {
        let output = step("clippy")
            .command(repo.root())
            .env("CARGO_TARGET_DIR", repo.root().join("target"))
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        (output.status.success(), stderr)
    }

    fn assert_clippy_rejects(repo: &TestRepo, expected: &str) {
        let (success, stderr) = clippy(repo);
        assert!(!success && stderr.contains(expected), "{stderr}");
    }

    /// The end of the help link clippy prints with a lint's first error. The
    /// newline keeps `allow_attributes` from matching a longer lint name.
    fn clippy_lint(name: &str) -> String {
        format!("index.html#{name}\n")
    }

    const SIMULATION_CRATES: [&str; 2] = ["ash-core", "ash-sim"];

    fn lib_rs(krate: &str) -> String {
        format!("crates/{krate}/src/lib.rs")
    }

    #[test]
    fn clippy_step_passes_on_a_copy_of_the_real_workspace() {
        let (success, stderr) = clippy(&scratch_workspace());
        assert!(success, "{stderr}");
    }

    #[test]
    fn float_arithmetic_is_rejected_in_simulation_crates() {
        for krate in SIMULATION_CRATES {
            let repo = scratch_workspace();
            repo.append(
                &lib_rs(krate),
                "pub fn mean(a: f32, b: f32) -> f32 {\n    (a + b) / 2.0\n}\n",
            );
            assert_clippy_rejects(&repo, &clippy_lint("float_arithmetic"));
        }
    }

    #[test]
    fn std_is_rejected_in_simulation_crates_outside_tests() {
        for krate in SIMULATION_CRATES {
            let repo = scratch_workspace();
            repo.append(
                &lib_rs(krate),
                "pub fn now() -> std::time::Instant {\n    std::time::Instant::now()\n}\n",
            );
            assert_clippy_rejects(&repo, "unlinked crate `std`");
        }
    }

    #[test]
    fn std_is_allowed_in_simulation_crate_tests() {
        let repo = scratch_workspace();
        repo.append(
            &lib_rs("ash-sim"),
            "#[cfg(test)]\nmod tests {\n    #[test]\n    fn clock() {\n        \
             let _ = std::time::Instant::now();\n    }\n}\n",
        );
        let (success, stderr) = clippy(&repo);
        assert!(success, "{stderr}");
    }

    #[test]
    fn integer_arithmetic_is_rejected_in_simulation_crates() {
        for krate in SIMULATION_CRATES {
            let repo = scratch_workspace();
            repo.append(
                &lib_rs(krate),
                "pub fn next(turn: u64) -> u64 {\n    turn + 1\n}\n",
            );
            assert_clippy_rejects(&repo, &clippy_lint("arithmetic_side_effects"));
        }
    }

    #[test]
    fn as_conversion_is_rejected() {
        let repo = scratch_workspace();
        repo.append(
            &lib_rs("ash-core"),
            "pub fn widen(x: u8) -> u32 {\n    x as u32\n}\n",
        );
        assert_clippy_rejects(&repo, &clippy_lint("as_conversions"));
    }

    #[test]
    fn allow_attribute_is_rejected_even_with_a_reason() {
        let repo = scratch_workspace();
        repo.append(
            &lib_rs("ash-core"),
            "#[allow(dead_code, reason = \"unused\")]\nfn unused() {}\n",
        );
        assert_clippy_rejects(&repo, &clippy_lint("allow_attributes"));
    }

    #[test]
    fn inner_allow_attribute_without_reason_is_rejected() {
        let repo = scratch_workspace();
        repo.append(
            &lib_rs("ash-core"),
            "mod quiet {\n    #![allow(dead_code)]\n    fn unused() {}\n}\n",
        );
        assert_clippy_rejects(&repo, &clippy_lint("allow_attributes_without_reason"));
    }

    #[test]
    fn unsafe_code_is_rejected() {
        let repo = scratch_workspace();
        repo.append(
            &lib_rs("ash-core"),
            "pub fn first(bytes: &[u8]) -> u8 {\n    unsafe { *bytes.as_ptr() }\n}\n",
        );
        assert_clippy_rejects(&repo, "`-F unsafe-code`");
    }

    #[test]
    fn mod_rs_file_is_rejected() {
        let repo = scratch_workspace();
        repo.append(&lib_rs("ash-core"), "mod world;\n");
        repo.write("crates/ash-core/src/world/mod.rs", "");
        assert_clippy_rejects(&repo, &clippy_lint("mod_module_files"));
    }

    #[test]
    fn simulation_crates_keep_overflow_checks_in_release() {
        let repo = scratch_workspace();
        let output = Command::new(cargo())
            .args([
                "build",
                "--release",
                "--verbose",
                "--package",
                "ash-core",
                "--package",
                "ash-sim",
            ])
            .current_dir(repo.root())
            .env("CARGO_TARGET_DIR", repo.root().join("target"))
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{stderr}");
        for krate in ["ash_core", "ash_sim"] {
            let rustc = stderr
                .lines()
                .find(|line| line.contains(&format!("--crate-name {krate} ")))
                .unwrap_or_else(|| panic!("no rustc call for {krate}: {stderr}"));
            assert!(rustc.contains("overflow-checks=on"), "{rustc}");
        }
    }

    #[test]
    fn missing_cargo_plugin_fails_with_install_command() {
        let error =
            require_subcommand("xtask-test-missing-plugin", "cargo install it").unwrap_err();
        assert!(error.contains("cargo install it"), "{error}");
    }
}
