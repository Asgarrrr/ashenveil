//! The dependency matrix: the only direct dependencies, of any kind, that each
//! workspace crate may declare. Dependencies point down toward `ash-core`;
//! only `ash-server` assembles. A new edge out of `ash-core` or `ash-sim`
//! requires an ADR. A crate missing from this list fails `verify`.

pub const ALLOWED: &[(&str, &[&str])] = &[
    ("ash-core", &[]),
    ("ash-sim", &["ash-core"]),
    ("xtask", &["serde_json"]),
];
