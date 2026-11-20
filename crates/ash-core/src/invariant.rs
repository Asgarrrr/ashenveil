//! The single way to crash on a violated invariant. The crash stops only the
//! world that hit it: the server quarantines that world.

/// Panics with `invariant violated: ` followed by the formatted message.
#[macro_export]
macro_rules! invariant {
    ($($message:tt)+) => {
        ::core::panic!("invariant violated: {}", ::core::format_args!($($message)+))
    };
}
