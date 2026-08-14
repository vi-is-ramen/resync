#[allow(unused)]
macro_rules! x {
    ($a:literal $b:ident) => {
        #[cfg(target_os = $a)]
        mod $b;
        #[cfg(target_os = $a)]
        pub use $b::*;
    };
}

x!("linux" linux);
x!("windows" windows);
x!("macos" macos);

/// Fallback atomic implementation as this platform isn't supported by Resync.
/// Same as [`crate::lock::Atomic`], which implements both [`crate::ILock`]
/// and [`crate::IShare`].
#[cfg(not(any(
    target_os = "linux",
    target_os = "windows",
    target_os = "macos",
)))]
pub type Os = crate::lock::Atomic;
