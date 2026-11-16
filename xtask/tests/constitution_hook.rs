//! The exit code is the contract with Claude Code: 2 blocks the tool call,
//! 0 lets it through. Only the real binary proves which one it returns.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// A checkout with a `.git` directory and a copy of the real list, deleted on drop.
struct Checkout(PathBuf);

impl Checkout {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("xtask-hook-exit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::create_dir_all(root.join(".claude")).unwrap();
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let list = ".claude/protected-paths.txt";
        std::fs::copy(repository.join(list), root.join(list)).unwrap();
        Self(root)
    }
}

impl Drop for Checkout {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn hook_exit_code(checkout: &Checkout, relative: &str) -> Option<i32> {
    let input = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "cwd": checkout.0,
        "tool_name": "Write",
        "tool_input": { "file_path": checkout.0.join(relative), "content": "x" },
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["hook", "constitution"])
        .stdin(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.to_string().as_bytes())
        .unwrap();
    child.wait().unwrap().code()
}

#[test]
fn protected_path_exits_2_and_free_path_exits_0() {
    let checkout = Checkout::new();
    assert_eq!(
        hook_exit_code(&checkout, "docs/adr/0001-data-model.md"),
        Some(2)
    );
    assert_eq!(
        hook_exit_code(&checkout, "crates/ash-sim/src/world.rs"),
        Some(0)
    );
}
