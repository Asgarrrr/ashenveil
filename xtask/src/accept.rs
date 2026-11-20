//! `cargo xtask accept <test>`: runs one named acceptance test across the
//! workspace (methodology, section 9: the proof of a slice).

use std::path::Path;
use std::process::ExitCode;

pub fn run(root: &Path, test: &str) -> ExitCode {
    let _ = (root, test);
    unimplemented!("spec 0001, slice S1")
}
