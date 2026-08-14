macro_rules! x {
    ($a:literal $b:ident) => {
        #[cfg(target_os = $a)]
        mod $b;
        #[cfg(target_os = $a)]
        pub use $b::*;
    };
}

x!("linux" linux);

/// Fallback stub implementation as this platform isn't supported by Resync.
/// Same as [`crate::park::Stub`].
#[cfg(not(any(
// list of supported platforms:
    target_os = "linux",
    // target_os = "windows",
    // target_os = "macos",
)))]
pub type Os = crate::park::Stub;
