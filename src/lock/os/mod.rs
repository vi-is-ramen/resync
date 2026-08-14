#[allow(unused)]
macro_rules! x {
    ($a:literal $b:ident) => {
        #[cfg(target_os = $a)]
        mod $b;
        #[cfg(target_os = $a)]
        pub use $b::*;
    };
}

/// Fallback atomic implementation as this platform isn't supported by Resync.
/// Same as [`crate::lock::Atomic`].
#[cfg(not(any(
// list of supported platforms:
    // target_os = "linux",
    // target_os = "windows",
    // target_os = "macos",
)))]
pub type Os = crate::lock::Atomic;
