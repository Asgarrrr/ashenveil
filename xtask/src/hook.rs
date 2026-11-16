//! The hooks behind `cargo xtask hook <name>`. Git and Claude Code call them
//! through launchers: `cargo xtask setup` installs the git ones, and
//! `.claude/settings.json` declares the Claude Code one.

pub mod commit_msg;
pub mod constitution;
